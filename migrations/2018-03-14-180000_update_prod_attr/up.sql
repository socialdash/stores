ALTER TABLE prod_attr_values ADD COLUMN base_prod_id INTEGER NOT NULL REFERENCES base_products (id);