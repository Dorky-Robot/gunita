ALTER TABLE memory_items ADD COLUMN cid TEXT;
CREATE INDEX idx_memory_items_cid ON memory_items(cid);
