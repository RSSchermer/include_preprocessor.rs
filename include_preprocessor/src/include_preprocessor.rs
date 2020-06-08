use nom::branch::alt;
use nom::bytes::complete::{is_not, tag};
use nom::character::complete::{char, line_ending, not_line_ending, space0, space1};
use nom::combinator::opt;
use nom::lib::std::collections::HashSet;
use nom::sequence::{delimited, tuple};
use nom::IResult;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Error as IOError;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::{fs, mem, slice};
use threadpool::ThreadPool;

pub struct SearchPaths {
    base_paths: Vec<PathBuf>,
    quoted_paths: Vec<PathBuf>,
}

impl SearchPaths {
    pub fn new() -> Self {
        SearchPaths {
            base_paths: Vec::new(),
            quoted_paths: Vec::new(),
        }
    }

    pub fn push_base_path<P>(&mut self, path: P)
    where
        P: AsRef<Path>,
    {
        let mut buf = PathBuf::new();

        buf.push(path);

        self.base_paths.push(buf);
    }

    pub fn push_quoted_path<P>(&mut self, path: P)
    where
        P: AsRef<Path>,
    {
        let mut buf = PathBuf::new();

        buf.push(path);

        self.quoted_paths.push(buf);
    }

    pub fn base_paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.base_paths.iter()
    }

    pub fn quoted_paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.quoted_paths.iter().chain(self.base_paths.iter())
    }
}

pub enum Error {
    FileNotFound(FileNotFoundError),
    IO(IOError),
    Parse(ParseError),
}

impl From<FileNotFoundError> for Error {
    fn from(err: FileNotFoundError) -> Self {
        Error::FileNotFound(err)
    }
}

impl From<IOError> for Error {
    fn from(err: IOError) -> Self {
        Error::IO(err)
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Error::Parse(err)
    }
}

#[derive(Debug)]
pub struct FileNotFoundError {
    included_path: PathBuf,
}

#[derive(Debug)]
pub struct ParseError {
    source_file: PathBuf,
    line_number: usize,
    message: String,
}

pub fn preprocess<P, W>(
    entry_point: P,
    search_paths: SearchPaths,
    mut writer: W,
) -> Result<W, Error>
where
    P: AsRef<Path>,
    W: Writer,
{
    let parsed = Parsed::try_init(entry_point, search_paths)?;

    parsed.write(&mut writer);

    Ok(writer)
}

enum LoadState {
    Loaded(ParsedNode),
    Pending,
}

impl LoadState {
    fn loaded(&self) -> Option<&ParsedNode> {
        if let LoadState::Loaded(node) = self {
            Some(node)
        } else {
            None
        }
    }
}

struct Parsed {
    lookup: HashMap<u64, LoadState>,
    root_key: u64,
}

impl Parsed {
    fn try_init<P>(entry_point: P, search_paths: SearchPaths) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let mut lookup = HashMap::new();
        let (tx, rx) = mpsc::channel();
        let pool = ThreadPool::new(num_cpus::get());

        let mut hasher = DefaultHasher::new();

        entry_point.as_ref().hash(&mut hasher);

        let root_key = hasher.finish();
        let root_node = ParsedNode::try_parse(entry_point, &search_paths);

        lookup.insert(root_key, LoadState::Pending);

        tx.send(root_node).unwrap();

        let search_paths = Arc::new(search_paths);
        let mut balance = 1;

        loop {
            if balance == 0 {
                break;
            }

            let node = rx.recv().unwrap()?;

            balance -= 1;

            // Load and parse any files included by this node.
            'inner: for chunk in node.chunks() {
                if let NodeChunk::Include(path) = chunk {
                    let mut hasher = DefaultHasher::new();

                    path.hash(&mut hasher);

                    let key = hasher.finish();

                    if lookup.contains_key(&key) {
                        // File has been/is being loaded, skip
                        continue 'inner;
                    }

                    // Not yet loaded, try and load
                    lookup.insert(key, LoadState::Pending);
                    balance += 1;

                    let tx_clone = tx.clone();
                    let search_paths_clone = search_paths.clone();
                    let path_buf = path.to_path_buf();

                    pool.execute(move || {
                        tx_clone
                            .send(ParsedNode::try_parse(path_buf, &search_paths_clone))
                            .unwrap();
                    });
                }
            }

            lookup.insert(node.key(), LoadState::Loaded(node));
        }

        Ok(Parsed { lookup, root_key })
    }

    fn get_by_key(&self, key: u64) -> Option<&ParsedNode> {
        self.lookup.get(&key).and_then(|node| node.loaded())
    }

    fn get_by_path<P>(&self, path: P) -> Option<&ParsedNode>
    where
        P: AsRef<Path>,
    {
        let mut hasher = DefaultHasher::new();

        path.as_ref().hash(&mut hasher);

        let key = hasher.finish();

        self.get_by_key(key)
    }

    fn write<W>(&self, writer: &mut W)
    where
        W: Writer,
    {
        let mut stack = Vec::new();
        let mut seen = HashSet::new();

        let root_node = self.get_by_key(self.root_key).unwrap();

        if root_node.once() {
            seen.insert(root_node.key());
        }

        let mut current_node = root_node;
        let mut current_chunk = 0;

        loop {
            if let Some(chunk) = current_node.get_chunk(current_chunk) {
                match chunk {
                    NodeChunk::Text(chunk) => {
                        writer.write_chunk(chunk);

                        current_chunk += 1;
                    }
                    NodeChunk::Include(path) => {
                        stack.push((current_node.key(), current_chunk));

                        let node = self.get_by_path(path).unwrap();

                        if node.once() && seen.contains(&node.key()) {
                            current_chunk += 1;
                        } else {
                            seen.insert(node.key());

                            current_node = node;
                            current_chunk = 0;
                        }
                    }
                }
            } else {
                if let Some((parent_key, child_chunk)) = stack.pop() {
                    current_node = self.get_by_key(parent_key).unwrap();
                    current_chunk = child_chunk + 1;
                } else {
                    break;
                }
            }
        }
    }
}

enum NodeChunkInternal {
    Text(Range<usize>),
    Include(PathBuf),
}

enum NodeChunk<'a> {
    Text(&'a str),
    Include(&'a Path),
}

struct ParsedNode {
    key: u64,
    once: bool,
    source: String,
    chunk_buffer: Vec<NodeChunkInternal>,
}

impl ParsedNode {
    fn try_parse<P>(path: P, search_paths: &SearchPaths) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let source = fs::read_to_string(&path)?;
        let source_len = source.len();
        let parent_path = path.as_ref().parent().unwrap();

        let mut remainder = source.as_str();
        let mut line_number = 0;
        let mut chunk_buffer = Vec::new();
        let mut once = false;
        let mut current_text_range = 0..0;

        while remainder.len() > 0 {
            let (new_remainder, line) = parse_line(remainder).map_err(|err| {
                let mut buf = PathBuf::new();

                buf.push(&path);

                ParseError {
                    source_file: buf,
                    line_number,
                    message: err.to_string(),
                }
            })?;

            let pos = source_len - new_remainder.len();

            if line != Line::Text {
                if current_text_range.len() != 0 {
                    let range = mem::replace(&mut current_text_range, pos..pos);

                    chunk_buffer.push(NodeChunkInternal::Text(range))
                }
            } else {
                current_text_range.end = pos;
            }

            match line {
                Line::Include(path) => {
                    let path = path.try_resolve(parent_path, search_paths)?;

                    chunk_buffer.push(NodeChunkInternal::Include(path));
                }
                Line::PragmaOnce => {
                    once = true;
                }
                _ => (),
            }

            remainder = new_remainder;
            line_number += 1;
        }

        if current_text_range.len() != 0 {
            chunk_buffer.push(NodeChunkInternal::Text(current_text_range))
        }

        let mut hasher = DefaultHasher::new();

        path.as_ref().hash(&mut hasher);

        let key = hasher.finish();

        Ok(ParsedNode {
            key,
            once,
            source,
            chunk_buffer,
        })
    }

    fn key(&self) -> u64 {
        self.key
    }

    fn once(&self) -> bool {
        self.once
    }

    fn get_chunk(&self, index: usize) -> Option<NodeChunk> {
        self.chunk_buffer.get(index).map(|chunk| match chunk {
            NodeChunkInternal::Text(range) => NodeChunk::Text(&self.source[range.clone()]),
            NodeChunkInternal::Include(path) => NodeChunk::Include(path.as_path()),
        })
    }

    fn chunks<'a>(&'a self) -> NodeChunks {
        let ParsedNode {
            source,
            chunk_buffer,
            ..
        } = self;

        NodeChunks {
            source,
            chunks: chunk_buffer.iter(),
        }
    }
}

struct NodeChunks<'a> {
    source: &'a String,
    chunks: slice::Iter<'a, NodeChunkInternal>,
}

impl<'a> Iterator for NodeChunks<'a> {
    type Item = NodeChunk<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let NodeChunks { source, chunks } = self;

        if let Some(chunk) = chunks.next() {
            let chunk = match chunk {
                NodeChunkInternal::Text(range) => NodeChunk::Text(&source[range.clone()]),
                NodeChunkInternal::Include(path) => NodeChunk::Include(path),
            };

            Some(chunk)
        } else {
            None
        }
    }
}

pub trait Writer {
    fn write_chunk(&mut self, chunk: &str);
}

pub struct StringWriter {
    output_buffer: String,
}

impl StringWriter {
    pub fn new() -> Self {
        StringWriter {
            output_buffer: String::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        StringWriter {
            output_buffer: String::with_capacity(capacity),
        }
    }

    pub fn inspect(&self) -> &str {
        &self.output_buffer
    }
}

impl Into<String> for StringWriter {
    fn into(self) -> String {
        self.output_buffer
    }
}

impl Writer for StringWriter {
    fn write_chunk(&mut self, chunk: &str) {
        self.output_buffer.push_str(chunk);
    }
}

fn parse_line(input: &str) -> IResult<&str, Line> {
    alt((parse_include, parse_pragma_once, parse_text))(input)
}

fn parse_text(input: &str) -> IResult<&str, Line> {
    let (rem, _) = tuple((not_line_ending, opt(line_ending)))(input)?;

    Ok((rem, Line::Text))
}

fn parse_include(input: &str) -> IResult<&str, Line> {
    let (rem, (_, _, path, _, _)) = tuple((
        tag("#include"),
        space1,
        parse_include_path,
        space0,
        line_ending,
    ))(input)?;

    Ok((rem, Line::Include(path)))
}

fn parse_include_path(input: &str) -> IResult<&str, IncludePath> {
    alt((parse_angle_path, parse_quote_path))(input)
}

fn parse_angle_path(input: &str) -> IResult<&str, IncludePath> {
    let (rem, target) = delimited(char('<'), is_not(">"), char('>'))(input)?;

    Ok((rem, IncludePath::Angle(target.as_ref())))
}

fn parse_quote_path(input: &str) -> IResult<&str, IncludePath> {
    let (rem, target) = delimited(char('"'), is_not("\""), char('"'))(input)?;

    Ok((rem, IncludePath::Quote(target.as_ref())))
}

fn parse_pragma_once(input: &str) -> IResult<&str, Line> {
    let (rem, _) = tuple((tag("#pragma"), space1, tag("once"), space0, line_ending))(input)?;

    Ok((rem, Line::PragmaOnce))
}

#[derive(PartialEq, Debug)]
enum Line<'a> {
    Text,
    Include(IncludePath<'a>),
    PragmaOnce,
}

#[derive(PartialEq, Debug)]
enum IncludePath<'a> {
    Angle(&'a Path),
    Quote(&'a Path),
}

impl<'a> IncludePath<'a> {
    fn try_resolve(
        &self,
        current_parent: &Path,
        search_paths: &SearchPaths,
    ) -> Result<PathBuf, FileNotFoundError> {
        match self {
            IncludePath::Angle(path) => {
                for search_path in search_paths.base_paths() {
                    let join = search_path.join(path);

                    if join.is_file() {
                        return Ok(join);
                    }
                }

                Err(FileNotFoundError {
                    included_path: path.to_path_buf(),
                })
            }
            IncludePath::Quote(path) => {
                let join = current_parent.join(path);

                if join.is_file() {
                    return Ok(join);
                }

                for search_path in search_paths.quoted_paths() {
                    let join = search_path.join(path);

                    if join.is_file() {
                        return Ok(join);
                    }
                }

                Err(FileNotFoundError {
                    included_path: path.to_path_buf(),
                })
            }
        }
    }
}
