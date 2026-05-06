use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const MAGIC: &[u8; 4] = b"TSQL";
const VERSION: u8 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColType {
    Int,
    Text,
}

#[derive(Clone, Debug)]
pub struct Column {
    pub name: String,
    pub ty: ColType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    Text(String),
}

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    BadMagic,
    BadVersion,
    BadData(String),
    SchemaAlreadySet,
    NoSchema,
    WrongArity { expected: usize, got: usize },
    TypeMismatch { col: usize, expected: &'static str },
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        StorageError::Io(e)
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Io(e) => write!(f, "I/O error: {}", e),
            StorageError::BadMagic => write!(f, "Bad magic number"),
            StorageError::BadVersion => write!(f, "Bad version number"),
            StorageError::BadData(e) => write!(f, "Bad data: {}", e),
            StorageError::SchemaAlreadySet => write!(f, "Schema already set"),
            StorageError::NoSchema => write!(f, "No schema"),
            StorageError::WrongArity { expected, got } => write!(f, "Wrong arity: expected {}, got {}", expected, got),
            StorageError::TypeMismatch { col, expected } => write!(f, "Type mismatch: column {}, expected {}", col, expected),
        }
    }
}


impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StorageError::Io(e) => Some(e),
            _ => None,
        }
    }
}

pub struct Storage {
    file: File,
    schema: Option<Vec<Column>>,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let len = file.metadata()?.len();
        let schema = if len == 0 {
            None
        } else {
            Some(Self::read_schema(&mut file)?)
        };

        Ok(Storage { file, schema })
    }

    fn read_schema(file: &mut File) -> Result<Vec<Column>, StorageError> {
        file.seek(SeekFrom::Start(0))?;
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(StorageError::BadMagic);
        }
        let mut ver = [0u8; 1];
        file.read_exact(&mut ver)?;
        if ver[0] != VERSION {
            return Err(StorageError::BadVersion);
        }

        let ncols = read_u16(file)? as usize;
        let mut cols = Vec::with_capacity(ncols);
        for _ in 0..ncols {
            let name = read_string(file)?;
            let mut tyb = [0u8; 1];
            file.read_exact(&mut tyb)?;
            let ty = match tyb[0] {
                1 => ColType::Int,
                2 => ColType::Text,
                _ => return Err(StorageError::BadData("unknown column type".into())),
            };
            cols.push(Column { name, ty });
        }
        Ok(cols)
    }

    pub fn schema(&self) -> Option<&[Column]> {
        self.schema.as_deref()
    }

    /// Writes magic, version, schema. Fails if schema already exists on disk.
    pub fn init_schema(&mut self, columns: Vec<Column>) -> Result<(), StorageError> {
        if self.schema.is_some() {
            return Err(StorageError::SchemaAlreadySet);
        }
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(MAGIC)?;
        self.file.write_all(&[VERSION])?;
        write_u16(&mut self.file, u16::try_from(columns.len()).map_err(|_| {
            StorageError::BadData("too many columns".into())
        })?)?;
        for c in &columns {
            write_string(&mut self.file, &c.name)?;
            let ty_byte: u8 = match c.ty {
                ColType::Int => 1,
                ColType::Text => 2,
            };
            self.file.write_all(&[ty_byte])?;
        }
        self.file.sync_all()?;
        self.schema = Some(columns);
        Ok(())
    }

    fn schema_ref(&self) -> Result<&[Column], StorageError> {
        self.schema.as_deref().ok_or(StorageError::NoSchema)
    }

    pub fn append_row(&mut self, values: &[Value]) -> Result<(), StorageError> {
        let schema = self.schema_ref()?.to_vec();
        if values.len() != schema.len() {
            return Err(StorageError::WrongArity {
                expected: schema.len(),
                got: values.len(),
            });
        }
        for (i, (col, val)) in schema.iter().zip(values.iter()).enumerate() {
            match (&col.ty, val) {
                (ColType::Int, Value::Int(_)) | (ColType::Text, Value::Text(_)) => {}
                (ColType::Int, _) => {
                    return Err(StorageError::TypeMismatch {
                        col: i,
                        expected: "INT",
                    })
                }
                (ColType::Text, _) => {
                    return Err(StorageError::TypeMismatch {
                        col: i,
                        expected: "TEXT",
                    })
                }
            }
        }

        let payload = encode_row(values);
        let len_u32 = u32::try_from(payload.len())
            .map_err(|_| StorageError::BadData("row too large".into()))?;

        self.file.seek(SeekFrom::End(0))?;
        write_u32(&mut self.file, len_u32)?;
        self.file.write_all(&payload)?;
        self.file.sync_all()?;
        Ok(())
    }

    pub fn scan_rows(&mut self) -> Result<Vec<Vec<Value>>, StorageError> {
        let schema = self.schema_ref()?.to_vec();
        self.file.seek(SeekFrom::Start(0))?;
        let mut magic = [0u8; 4];
        self.file.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(StorageError::BadMagic);
        }
        let mut ver = [0u8; 1];
        self.file.read_exact(&mut ver)?;
        if ver[0] != VERSION {
            return Err(StorageError::BadVersion);
        }
        let ncols = read_u16(&mut self.file)? as usize;
        for _ in 0..ncols {
            let _name = read_string(&mut self.file)?;
            let mut tyb = [0u8; 1];
            self.file.read_exact(&mut tyb)?;
        }

        let mut out = Vec::new();
        loop {
            let mut len_buf = [0u8; 4];
            match self.file.read_exact(&mut len_buf) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }
            let row_len = u32::from_le_bytes(len_buf) as usize;
            let mut buf = vec![0u8; row_len];
            self.file.read_exact(&mut buf)?;
            out.push(decode_row(&buf, &schema)?);
        }
        Ok(out)
    }
}

fn encode_row(values: &[Value]) -> Vec<u8> {
    let mut v = Vec::new();
    for val in values {
        match val {
            Value::Int(x) => {
                v.push(1);
                v.extend_from_slice(&x.to_le_bytes());
            }
            Value::Text(s) => {
                v.push(2);
                let b = s.as_bytes();
                let n = u32::try_from(b.len()).expect("text length");
                v.extend_from_slice(&n.to_le_bytes());
                v.extend_from_slice(b);
            }
        }
    }
    v
}

fn decode_row(mut bytes: &[u8], schema: &[Column]) -> Result<Vec<Value>, StorageError> {
    let mut row = Vec::with_capacity(schema.len());
    for col in schema {
        if bytes.is_empty() {
            return Err(StorageError::BadData("truncated row".into()));
        }
        let tag = bytes[0];
        bytes = &bytes[1..];
        match (&col.ty, tag) {
            (ColType::Int, 1) => {
                if bytes.len() < 8 {
                    return Err(StorageError::BadData("truncated int".into()));
                }
                let arr: [u8; 8] = bytes[0..8].try_into().unwrap();
                bytes = &bytes[8..];
                row.push(Value::Int(i64::from_le_bytes(arr)));
            }
            (ColType::Text, 2) => {
                if bytes.len() < 4 {
                    return Err(StorageError::BadData("truncated text len".into()));
                }
                let n = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
                bytes = &bytes[4..];
                if bytes.len() < n {
                    return Err(StorageError::BadData("truncated text".into()));
                }
                let s = std::str::from_utf8(&bytes[..n])
                    .map_err(|_| StorageError::BadData("invalid utf8".into()))?;
                bytes = &bytes[n..];
                row.push(Value::Text(s.to_string()));
            }
            _ => return Err(StorageError::BadData("tag/type mismatch".into())),
        }
    }
    if !bytes.is_empty() {
        return Err(StorageError::BadData("trailing row bytes".into()));
    }
    Ok(row)
}

fn read_u16(r: &mut File) -> Result<u16, StorageError> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(u16::from_le_bytes(b))
}

fn write_u16(w: &mut File, n: u16) -> Result<(), StorageError> {
    w.write_all(&n.to_le_bytes())?;
    Ok(())
}

fn write_u32(w: &mut File, n: u32) -> Result<(), StorageError> {
    w.write_all(&n.to_le_bytes())?;
    Ok(())
}

fn read_string(r: &mut File) -> Result<String, StorageError> {
    let n = read_u16(r)? as usize;
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|_| StorageError::BadData("invalid utf8 name".into()))
}

fn write_string(w: &mut File, s: &str) -> Result<(), StorageError> {
    let b = s.as_bytes();
    let n = u16::try_from(b.len()).map_err(|_| StorageError::BadData("name too long".into()))?;
    write_u16(w, n)?;
    w.write_all(b)?;
    Ok(())
}