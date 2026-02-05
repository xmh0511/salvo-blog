-- Performance Optimization: Add database indexes
-- These indexes will significantly improve query performance

-- Critical indexes for article_tb
-- Used in every page load with filtering and ordering
CREATE INDEX IF NOT EXISTS idx_article_state_level ON article_tb(article_state, level);
CREATE INDEX IF NOT EXISTS idx_article_update_time ON article_tb(update_time DESC);
CREATE INDEX IF NOT EXISTS idx_article_user_id ON article_tb(user_id);

-- Critical indexes for view_tb
-- Used for counting views per article
CREATE INDEX IF NOT EXISTS idx_view_article_id ON view_tb(article_id);

-- Critical indexes for comment_tb
-- Used for counting comments per article
CREATE INDEX IF NOT EXISTS idx_comment_article_id ON comment_tb(article_id);
CREATE INDEX IF NOT EXISTS idx_comment_user_id ON comment_tb(user_id);

-- Indexes for tag_tb
-- Used in LIKE searches
CREATE INDEX IF NOT EXISTS idx_tag_name ON tag_tb(name(50));

-- Indexes for user_tb
-- Used in registration checks and searches
CREATE INDEX IF NOT EXISTS idx_user_name ON user_tb(name(50));
CREATE INDEX IF NOT EXISTS idx_user_email ON user_tb(email(100));

-- Show the newly created indexes
SHOW INDEX FROM article_tb;
SHOW INDEX FROM view_tb;
SHOW INDEX FROM comment_tb;
