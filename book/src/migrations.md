# Migrations

Ergol comes with a (pretty simple for the moment) system for migrations.

In order to be able to manage your migrations, you need to install ergol cli:

```sh
cargo install ergol_cli
```

## Saving migrations

When building your application, the `ergol` proc macro automatically generates
json files for each marked structure. Those json files represent the state of
the database, and are stored in the `migrations/current` directory.

You can freeze a certain state of your database by running

```sh
ergol save
```

It will copy the `migrations/current` directory to a `migrations/0` directory
(or `migrations/n` _n_ being the number of migrations you already have). It will
also add `up.sql` and `down.sql` to migrate from _n - 1_ to _n_ and back.

**Note:** when adding new columns, you probably will need to specify default
values by editing directly the `migrations/n/up.sql`.

For example, let's say I start an application with the following model:
```rust
# extern crate ergol;
# use ergol::prelude::*;
#[ergol]
pub struct user {
    #[id] id: i32,
    username: string,
    email: string,
}
```

The `up.sql` will look like this
```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR NOT NULL,
    email VARCHAR NOT NULL
);
```
and the `down.sql` will look like this
```sql
DROP TABLE users CASCADE;
```

I build my application, then run `ergol save`, and have my
`migrations/0/{up.sql,down.sql}` files.

Let's say I want to add a new attribute to my `User` struct for the age of the
users.

```rust
# extern crate ergol;
# use ergol::prelude::*;
#[ergol]
pub struct user {
    #[id] id: i32,
    username: string,
    email: string,
    age: i32,
}
```

If I run `ergol save` again, I will have a `migrations/1/up.sql` that will look
like this:
```sql
ALTER TABLE users ADD age INT NOT NULL DEFAULT /* TODO default value */;
```
You need to change this code to set the default value for the column.

Running `ergol hint` will show the code that migrates from the last migration
to the current migration.

## Running migrations

You can run all migrations by running `ergol migrate`. Ergol creates a special
table that it uses to keep track of what migration the database is in, and will
run all migrations between the current migration of the database to the last
saved migrations.

## Reset

The last useful command you can do with ergol is `ergol reset`. It deletes the
whole database, and recreate it using only the last migration. It is
particularly useful when developping an app when you want to reset your database.
