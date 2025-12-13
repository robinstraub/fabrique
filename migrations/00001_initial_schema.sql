CREATE TABLE anvils (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  material TEXT NOT NULL,
  name TEXT NOT NULL,
  weight SMALLINT NOT NULL,
  deleted_at TIMESTAMPTZ
);

CREATE TABLE orders (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid()
);

CREATE TABLE order_lines (
  anvil_id UUID NOT NULL REFERENCES anvils(id),
  order_id UUID NOT NULL REFERENCES orders(id),
  amount SMALLINT NOT NULL DEFAULT 1,
  PRIMARY KEY (order_id, anvil_id)
);
