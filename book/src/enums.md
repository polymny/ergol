# Enums in ergol

To use an enum in an `ergol` managed struct, this enum needs to derive
`Debug` and `PgEnum`. An `ergol` managed enum cannot have attributes.

```rust
# extern crate ergol;
use ergol::prelude::*;

#[ergol]
pub struct User {
    #[id] pub id: i32,
    #[unique] pub username: String,
    pub password: String,
    pub role: Role,
}

#[derive(PgEnum, Debug)]
pub enum Role {
    Guest,
    Admin,
}
```
