-- Migration 010: Add fit_mode column to cameras table
-- fit_mode: 'fit' = 画像全体を収める (object-fit: contain)
--           'trim' = カードを画像で埋める (object-fit: cover) - DEFAULT

ALTER TABLE cameras ADD COLUMN fit_mode VARCHAR(10) NOT NULL DEFAULT 'trim';
