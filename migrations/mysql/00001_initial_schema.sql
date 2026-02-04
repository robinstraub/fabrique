CREATE TABLE users (
    id BINARY(16) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    deleted_at DATETIME
);

CREATE TABLE addresses (
    id BINARY(16) PRIMARY KEY,
    user_id BINARY(16) NOT NULL,
    label VARCHAR(255) NOT NULL,
    street VARCHAR(255) NOT NULL,
    city VARCHAR(255) NOT NULL,
    zip VARCHAR(20) NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE products (
    id BINARY(16) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    price_cents INTEGER NOT NULL,
    in_stock BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE orders (
    id BINARY(16) PRIMARY KEY,
    user_id BINARY(16) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    deleted_at DATETIME,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE order_lines (
    order_id BINARY(16) NOT NULL,
    product_id BINARY(16) NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 1,
    unit_price_cents INTEGER NOT NULL,
    PRIMARY KEY (order_id, product_id),
    FOREIGN KEY (order_id) REFERENCES orders(id),
    FOREIGN KEY (product_id) REFERENCES products(id)
);

CREATE TABLE messages (
    id BINARY(16) PRIMARY KEY,
    content TEXT NOT NULL,
    sender_id BINARY(16) NOT NULL,
    recipient_id BINARY(16) NOT NULL,
    FOREIGN KEY (sender_id) REFERENCES users(id),
    FOREIGN KEY (recipient_id) REFERENCES users(id)
);
