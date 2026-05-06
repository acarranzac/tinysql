# 1. Relational Model & Algebra

Database structure:

- External Schema: Views (SQL)
- Logical Schema: Schema, Constraints... (SQL)
- Physical Schema: Pages, Files, Extents...
- Database Storage

Relational model:

- A relation is an unordered set that contains the relationship of attributes that represent entities.
- A tuple is a set of attributes values (aka its domain) in the relation.
- n-ary Relation = Table with n columns..

- A relations's primary key uniquely identifies a single tuple (identity column).
- A foreign key specifies that an attribute from one relation maps to a tuple in another relation.
- Constraints: user-defined conditions that must hold for any instance of the database.

Data Manipulation Languages (DML):

- Procedural (relational algebra) or non-procedural (declarative, relational calculus)
- Relational Algebra: 7 operators - Select, Projection, Union, Intersection, Difference, Product, Join
- Execution engine is going to look a lot like RA operators

# 2. Database Storage: Files and Pages

Layers:
- Query Planning: take SQL queries to convert them to physical plans to execute in the system.
- Operator Execution: executing queries
- Access Methods: Accessing data on behalf of operators
- Buffer Pool Manager: bringing the pages from disk to memory and handing them to components.
- Disk Manager: the actual db, files on disk

Background
- Disk-based architecture: primary is non-volatile disk, components manage the movement between non-volatile and volatile.
- Storage Hierarchy: Network Storage > HDD > SSD > DRAM > CPU Caches > CPU Registers (1st 3 non-volatile sequential access block-addressable and then volatile - random access byte-addressable)
- Access times: 1ns L1 Cache Ref, 4ns L2 Cache Ref, 100 ns DRAM, 16000ns SSD, 2M ns HD, 50M ns Network storage, 1000M ns tape archives.
- Sequential vs random access: random access on non-volatile storage is almost always slower than sequential access.
- DMBS will want to maximize sequential access. Algos try to recude number of writes to random pages so that data is stored in contiguous blocks. Allocating multiple pages at the same time is called an extent.
- System design goals: allow the dbms to manage databases that exceed the amount of memory available.
- Disk-oriented DBMS: DISK - Database file, directory pages with header / MEMORY - Buffer Pool
- Problems: How the DBMS represents the database in files on disk / How the DBMS manages its memory and moves data bach-and-forth from disk.

File Storage
- DBMS stores a db as 1 or more files on disk typically in a proprietary format (OS does not know anything about it)
- Storage manager: responsible for maintaining a database's files: organizes the files as collection of pages (tracks data read - written, available space, improve spatial and tempora locality of pages)
- Pages: a page is a fixed-size block of data, can contains tuples, metadata, indexes, log records, with a pageID
- 3 different notionsÑ hardware page 4 kb, OS page 4kb, x64 2mb/1gb, database page 512B-32KB
- read heavy larger page sizes (+1MB) and write heavy smaller page (4-16KB)
- Page storage architecture: Heap file, tree file, sequiential/sorted file, hasing file
- Heap file: unordered collection of pages with tuples that are stored in random order. Create / Get / Write / Delete Page, iterate pages.
- Page directory location of data pdb files, one entry per db object.

Page Layout
- Every page contains a header of metadata about the page´s contents (size, checksum, dbms version, transaction visibility, compression/encoding, schema info, summary, sketches)
- For any page storage architecture, we now need to decide how to organize data inside: 1- Tuple-oriented, 2- log-structured, 3-index-organized
- Tuple-oriented: typical layout scheme is slotted pages, the slot array maps slots to the tuples starting position offsets.
- Record IDs: applications should never rely on these to mean anything, only represent physical location.

Tuple Layout
- Sequence of bytes prefixed with a header that contains meta-data about it.
- Data: attributes typically stored in order (header: a, b, c, d , e)
- Word alignment (64-bit word): Add padding (empty bits after attributes to ensure that tuple is word aligned)
- Reordering also, but still might need padding (not many system do this)
- real, float/dobule (variable) are not as precise as numeric or decimal (fixed) for example.
- NULL data types: null column bitmap header (row stores) special values as placeholder (column stores)
- Large values: overflow storage pages 
- External value storage: treated as a BLOB type.

Database is organized in pages.
Different ways to track pages.
Different ways to store pages.
Different ways to store tuples.

Problems: how the DBMS represents the database in files on disk AND manages its memory and moves data back-and-forth from disk.

# 3. Memory Management and Buffer Pools

