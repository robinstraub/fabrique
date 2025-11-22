CREATE TABLE anvils (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  weight SMALLINT NOT NULL
);
