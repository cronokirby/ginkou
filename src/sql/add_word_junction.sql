INSERT INTO WordSentence
SELECT id, ?2 FROM WORDS WHERE word=?1 AND NOT EXISTS (SELECT 1 FROM WordSentence WHERE word_id=id AND sentence_id=?2);