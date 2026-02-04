PRAGMA foreign_keys = ON;

CREATE TABLE users (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    deleted_at TEXT
);

CREATE TABLE addresses (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    label TEXT NOT NULL,
    street TEXT NOT NULL,
    city TEXT NOT NULL,
    zip TEXT NOT NULL
);

CREATE TABLE products (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    price_cents INTEGER NOT NULL,
    in_stock BOOLEAN NOT NULL DEFAULT 1
);

CREATE TABLE orders (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'pending',
    deleted_at TEXT
);

CREATE TABLE order_lines (
    order_id TEXT NOT NULL REFERENCES orders(id),
    product_id TEXT NOT NULL REFERENCES products(id),
    quantity INTEGER NOT NULL DEFAULT 1,
    unit_price_cents INTEGER NOT NULL,
    PRIMARY KEY (order_id, product_id)
);

CREATE TABLE messages (
    id TEXT PRIMARY KEY NOT NULL,
    content TEXT NOT NULL,
    sender_id TEXT NOT NULL REFERENCES users(id),
    recipient_id TEXT NOT NULL REFERENCES users(id)
);
