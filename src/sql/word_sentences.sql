SELECT sentence FROM Sentences
LEFT JOIN WordSentence on WordSentence.word_id = sentences.id
LEFT JOIN Words on Words.id = WordSentence.word_id
WHERE word=?1