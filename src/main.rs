use std::io;
use std::path::Path;
use std::string::FromUtf8Error;
#[macro_use]
extern crate rusqlite;
use rusqlite::Connection;

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
    reader: R,
}

impl<B: io::BufRead> Iterator for Sentences<B> {
    type Item = Result<String, SentenceError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = Vec::new();
        for byte in &DAKUTEN_BYTES {
            let read = self.reader.read_until(*byte, &mut buf);
            if let Err(e) = read {
                return Some(Err(e.into()));
            }
            if read.unwrap() == 0 {
                return None;
            }
        }
        Some(String::from_utf8(buf).map_err(SentenceError::from))
    }
}

fn sentences<R: io::BufRead>(reader: R) -> Sentences<R> {
    Sentences { reader }
}

struct Bank {
    conn: Connection,
}

impl Bank {
    fn new() -> rusqlite::Result<Self> {
        let existed = Path::new("db").exists();
        let conn = Connection::open("db")?;
        if !existed {
            conn.execute_batch(include_str!("sql/setup.sql"))?;
        }
        Ok(Bank { conn })
    }

    fn add_sentence(&mut self, sentence: &str) -> rusqlite::Result<u32> {
        let add_sentence = include_str!("sql/add_sentence.sql");
        self.conn.execute(add_sentence, params![sentence])?;
        let mut stmt = self.conn.prepare("SELECT last_insert_rowid()")?;
        stmt.query_row(params![], |row| row.get(0))
    }

    fn add_word(&mut self, word: &str, sentence_id: u32) -> rusqlite::Result<()> {
        let add_word = include_str!("sql/add_word.sql");
        self.conn.execute(add_word, params![word])?;
        let junction = include_str!("sql/add_word_junction.sql");
        self.conn.execute(junction, params![word, sentence_id])?;
        Ok(())
    }

    fn matching_word(&mut self, word: &str) -> rusqlite::Result<Vec<String>> {
        let matching = include_str!("sql/word_sentences.sql");
        let mut stmt = self.conn.prepare(matching)?;
        let mut buffer = Vec::new();
        let results = stmt.query_map(params![word], |row| row.get(0))?;
        for r in results {
            let s: String = r?;
            buffer.push(s);
        }
        Ok(buffer)
    }
}

fn main() -> rusqlite::Result<()> {
    let mut bank = Bank::new()?;
    let s1 = bank.add_sentence("Hello World")?;
    bank.add_word("Hello", s1)?;
    bank.add_word("World", s1)?;
    let s2 = bank.add_sentence("Hello World again")?;
    bank.add_word("Hello", s2)?;
    bank.add_word("World", s2)?;
    bank.add_word("again", s2)?;
    println!("{:?}", bank.matching_word("Hello"));
    println!("{:?}", bank.matching_word("again"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::sentences;

    #[test]
    fn sentences_works_correctly() {
        let string = "A。B。XXC。";
        let mut iter = sentences(std::io::BufReader::new(string.as_bytes()));
        let a = iter.next();
        assert_eq!(String::from("A。"), a.unwrap().unwrap());
        let b = iter.next();
        assert_eq!(String::from("B。"), b.unwrap().unwrap());
        let c = iter.next();
        assert_eq!(String::from("XXC。"), c.unwrap().unwrap());
    }
}