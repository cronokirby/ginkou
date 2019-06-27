SELECT sentence FROM sentences
LEFT JOIN wordsentence ON wordsentence.sentence_id = sentences.id 
LEFT JOIN words ON words.id = wordsentence.word_id
WHERE word=?1
ORDER BY length(sentence)
LIMIT 200;