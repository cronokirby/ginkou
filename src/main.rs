use std::io;
use std::string::FromUtf8Error;
extern crate mecab;

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

fn main() {
    let chr = "。";
    println!("{:?}", chr.bytes().collect::<Vec<_>>());
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