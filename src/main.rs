use std::fs::File;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::string::FromUtf8Error;
extern crate dirs;
#[macro_use]
extern crate rusqlite;
use rusqlite::Connection;
extern crate structopt;
use structopt::StructOpt;
extern crate mecab;
use mecab::Tagger;


const DAKUTEN_BYTES: [u8; 3] = [227, 128, 130];

#[derive(Debug)]
enum SentenceError {
    Utf8(FromUtf8Error),
    IO(io::Error),
}

impl From<FromUtf8Error> for SentenceError {
    fn from(err: FromUtf8Error) -> Self {
        SentenceError::Utf8(err)
    }
}

impl From<io::Error> for SentenceError {
    fn from(err: io::Error) -> Self {
        SentenceError::IO(err)
    }
}

struct Sentences<R> {
    bytes: io::Bytes<R>,
    done: bool,
}

impl<B: io::BufRead> Iterator for Sentences<B> {
    type Item = Result<String, SentenceError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let mut buf = Vec::new();
        let mut match_index = 0;
        while match_index < 3 {
            let byte = match self.bytes.next() {
                None => break,
                Some(Err(e)) => return Some(Err(e.into())),
                Some(Ok(b)) => b,
            };
            buf.push(byte);
            if byte == DAKUTEN_BYTES[match_index] {
                match_index += 1;
            } else {
                match_index = 0;
            }
        }
        if buf.len() == 0 {
            self.done = true;
            return None;
        }
        let next = String::from_utf8(buf).map_err(SentenceError::from);
        Some(next.map(|x| x.replace(|x: char| x.is_whitespace(), "")))
    }
}

fn sentences<R: io::BufRead>(reader: R) -> Sentences<R> {
    Sentences {
        bytes: reader.bytes(),
        done: false,
    }
}


fn create_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(include_str!("sql/setup.sql"))
}

fn conn_from_disk<P: AsRef<Path>>(path: P) -> rusqlite::Result<Connection> {
    let existed = path.as_ref().exists();
    let conn = Connection::open(path)?;
    if !existed {
        create_tables(&conn)?;
    }
    Ok(conn)
}

fn conn_from_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    create_tables(&conn)?;
    Ok(conn)
}

fn add_sentence(conn: &Connection, sentence: &str) -> rusqlite::Result<u32> {
    let add_sentence = include_str!("sql/add_sentence.sql");
    conn.execute(add_sentence, params![sentence])?;
    Ok(conn.last_insert_rowid() as u32)
}

fn add_word(conn: &Connection, word: &str, sentence_id: u32) -> rusqlite::Result<()> {
    let add_word = include_str!("sql/add_word.sql");
    conn.execute(add_word, params![word])?;
    let junction = include_str!("sql/add_word_junction.sql");
    conn.execute(junction, params![word, sentence_id])?;
    Ok(())
}

fn matching_word(conn: &Connection, word: &str) -> rusqlite::Result<Vec<String>> {
    let matching = include_str!("sql/all_word_sentences.sql");
    let mut stmt = conn.prepare_cached(matching)?;
    let mut buffer = Vec::new();
    let results = stmt.query_map(params![word], |row| row.get(0))?;
    for r in results {
        let s: String = r?;
        buffer.push(s);
    }
    Ok(buffer)
}

// This will ignore broken pipes, to support unix piping into things like head
fn print_matching_words(conn: &Connection, word: &str, all: bool) -> rusqlite::Result<()> {
    let query = if all {
        include_str!("sql/all_word_sentences.sql")
    } else {
        include_str!("sql/best_word_sentences.sql")
    };
    let mut stmt = conn.prepare_cached(query)?;
    let results = stmt.query_map(params![word], |row| row.get(0))?;
    for r in results {
        let r: String = r?;
        if let Err(e) = write!(io::stdout(), "{}\n", r) {
            if e.kind() != io::ErrorKind::BrokenPipe {
                panic!(e);
            }
        }
    }
    Ok(())
}

fn consume_trimmed(conn: &Connection, trimmed: &str) -> rusqlite::Result<()> {
    let sentence_id = add_sentence(conn, trimmed)?;
    let mut tagger = Tagger::new("");
    tagger.parse_nbest_init(trimmed);
    let mecab_out = tagger.next().unwrap();
    for l in mecab_out.lines() {
        if l == "EOS" {
            break;
        }
        let tab_index = l.find('\t').unwrap();
        let (_, rest) = l.split_at(tab_index);
        // Remove the leading tab
        let rest = &rest[1..];
        let root = rest.split(',').skip(6).next().unwrap();
        add_word(conn, root, sentence_id)?;
    }
    Ok(())
}

fn consume_sentences<R: io::BufRead>(conn: &Connection, reader: R) -> rusqlite::Result<()> {
    let mut i = 0;
    for sentence in sentences(reader) {
        i += 1;
        if sentence.is_err() {
            println!("Err on #{}: {:?}", i, sentence);
            continue;
        };
        let sentence = sentence.unwrap();
        println!("#{}: {}", i, sentence);
        consume_trimmed(conn, &sentence)?;
    }
    Ok(())
}

#[derive(Debug, StructOpt)]
#[structopt(name = "ginkou", about = "Japanese sentence bank")]
enum Ginkou {
    /// Add new sentences to the database.
    #[structopt(name = "add")]
    Add {
        /// The file to read sentences from.
        ///
        /// If no file is given, sentences will be read from stdin.
        #[structopt(long, short = "f", parse(from_os_str))]
        file: Option<PathBuf>,
        /// The database to use.
        #[structopt(long = "database", short = "d", parse(from_os_str))]
        db: Option<PathBuf>,
    },
    /// Search for all sentences containing a given word.
    #[structopt(name = "get")]
    Get {
        /// The word to search for in the database.
        word: String,
        /// Show all results instead of shortest 200
        #[structopt(long = "allwords", short = "a")]
        all: bool,
        /// The database to use.
        #[structopt(long = "database", short = "d", parse(from_os_str))]
        db: Option<PathBuf>,
    },
}

fn default_db_path() -> PathBuf {
    if let Some(mut pb) = dirs::home_dir() {
        pb.push(".ginkoudb");
        pb
    } else {
        PathBuf::from(".ginkoudb")
    }
}

fn main() -> rusqlite::Result<()> {
    let opt = Ginkou::from_args();
    match opt {
        Ginkou::Get { word, all, db } => {
            let db_path = db.unwrap_or(default_db_path());
            let mut conn = conn_from_disk(&db_path)?;
            print_matching_words(&mut conn, &word, all)?;
        }
        Ginkou::Add { file, db } => {
            let db_path = db.unwrap_or(default_db_path());
            let mut conn = conn_from_disk(&db_path)?;
            let tx = conn.transaction()?;
            match file {
                None => {
                    consume_sentences(&tx, io::BufReader::new(io::stdin()))?;
                }
                Some(path) => {
                    let file_res = File::open(&path);
                    if let Err(e) = file_res {
                        println!("Couldn't open {}:\n {}", path.as_path().display(), e);
                        return Ok(());
                    }
                    let file = file_res.unwrap();
                    consume_sentences(&tx, io::BufReader::new(file))?;
                }
            };
            tx.commit()?;
        }
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentences_works_correctly() {
        let string = "A。\n  B。\n\n XXC。";
        let mut iter = sentences(std::io::BufReader::new(string.as_bytes()));
        let a = iter.next();
        assert_eq!(String::from("A。"), a.unwrap().unwrap());
        let b = iter.next();
        assert_eq!(String::from("B。"), b.unwrap().unwrap());
        let c = iter.next();
        assert_eq!(String::from("XXC。"), c.unwrap().unwrap());
    }

    #[test]
    fn bank_lookup_works_correctly() -> rusqlite::Result<()> {
        let conn = conn_from_memory()?;
        let sentence1 = String::from("A B");
        let sentence2 = String::from("A B C");
        let s1 = add_sentence(&conn, &sentence1)?;
        add_word(&conn, "A", s1)?;
        add_word(&conn, "B", s1)?;
        let s2 = add_sentence(&conn, &sentence2)?;
        add_word(&conn, "A", s2)?;
        add_word(&conn, "B", s2)?;
        add_word(&conn, "C", s2)?;
        let a_sentences = vec![sentence1.clone(), sentence2.clone()];
        assert_eq!(Ok(a_sentences), matching_word(&conn, "A"));
        let c_sentences = vec![sentence2.clone()];
        assert_eq!(Ok(c_sentences), matching_word(&conn, "C"));
        Ok(())
    }

    #[test]
    fn sentences_can_be_consumed() -> rusqlite::Result<()> {
        let conn = conn_from_memory()?;
        let sentence1 = "猫を見た";
        let sentence2 = "犬を見る";
        consume_trimmed(&conn, sentence1)?;
        consume_trimmed(&conn, sentence2)?;
        let a_sentences = vec![sentence1.into(), sentence2.into()];
        assert_eq!(Ok(a_sentences), matching_word(&conn, "見る"));
        let b_sentences = vec![sentence2.into()];
        assert_eq!(Ok(b_sentences), matching_word(&conn, "犬"));
        let c_sentences = vec![sentence1.into()];
        assert_eq!(Ok(c_sentences), matching_word(&conn, "猫"));
        Ok(())
    }
}