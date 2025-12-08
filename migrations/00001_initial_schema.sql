CREATE TABLE anvils (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  material TEXT NOT NULL,
  name TEXT NOT NULL,
  weight SMALLINT NOT NULL
);
