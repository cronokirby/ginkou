CREATE TABLE Words(
    id INTEGER PRIMARY KEY,
    word TEXT UNIQUE NOT NULL
);

CREATE TABLE Sentences(
    id INTEGER PRIMARY KEY,
    sentence TEXT NOT NULL
);

CREATE TABLE WordSentence(
    word_id INTEGER NOT NULL,
    sentence_id INTEGER NOT NULL,
    PRIMARY KEY(word_id, sentence_id),
    FOREIGN KEY(word_id) REFERENCES Words(id),
    FOREIGN KEY(sentence_id) REFERENCES sentences(id)
);