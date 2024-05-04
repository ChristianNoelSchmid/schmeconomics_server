CREATE TABLE users (
    id          INT     PRIMARY KEY,
    user_name   TEXT    NOT NULL, 
    acc_id      INT     NOT NULL
);

CREATE TABLE categories (
    id          INT     PRIMARY KEY,
    cat_name    TEXT    NOT NULL,

    bal         INT     NOT NULL,
    refill_val  INT     NOT NULL,
    order INT NOT NULL
);

CREATE TABLE transactions (
    id          INT     PRIMARY KEY,
    cat_id      INT     NOT NULL,
    user_id     INT     NOT NULL,

    am          INT  NOT NULL,
    is_refill   BIT  NOT NULL  DEFAULT 0,
    notes       TEXT,
    t_stamp     INT  NOT NULL  DEFAULT (cast(strftime('%s', 'now') as INT)),

    FOREIGN KEY (cat_id)    REFERENCES category,
    FOREIGN KEY (user_id)   REFERENCES user
);