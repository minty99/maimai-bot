-- Add jacket image URL columns for embeds.
ALTER TABLE scores ADD COLUMN jacket_url TEXT;
ALTER TABLE playlogs ADD COLUMN jacket_url TEXT;
