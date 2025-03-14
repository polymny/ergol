//! This module contains the types for postgres.

/// Any type that can be stored in a database should implement this trait.
pub trait Pg {
    /// Returns the potgres type corresponding to the type.
    fn ty() -> String;
}

impl Pg for String {
    fn ty() -> String {
        "VARCHAR NOT NULL".to_owned()
    }
}

impl Pg for i32 {
    fn ty() -> String {
        "INT NOT NULL".to_owned()
    }
}

impl Pg for i64 {
    fn ty() -> String {
        "BIGINT NOT NULL".to_owned()
    }
}

impl Pg for bool {
    fn ty() -> String {
        "BOOL NOT NULL".to_owned()
    }
}

impl Pg for f32 {
    fn ty() -> String {
        "REAL NOT NULL".to_owned()
    }
}

impl Pg for f64 {
    fn ty() -> String {
        "DOUBLE PRECISION NOT NULL".to_owned()
    }
}

impl<T: Pg + Send> Pg for Option<T> {
    fn ty() -> String {
        let current = T::ty();
        debug_assert!(current.ends_with(" NOT NULL"));
        return current[0..current.len() - 9].to_owned();
    }
}

#[allow(unused)]
macro_rules! impl_pg {
    ($ty: ty, $e: expr) => {
        impl Pg for $ty {
            fn ty() -> String {
                String::from($e)
            }
        }
    };

    ($ty: ty, $g: ident, $e: expr) => {
        impl<$g> Pg for $ty {
            fn ty() -> String {
                String::from($e)
            }
        }
    };
}

#[rustfmt::skip]
#[cfg(feature = "with-serde_json-1")]
impl_pg!(tokio_postgres::types::Json<T>, T, "JSON NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-bit-vec-0_6")]
impl_pg!(bit_vec::BitVec, "VARBIT NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-chrono-0_4")]
impl_pg!(chrono::NaiveDateTime, "TIMESTAMP NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-chrono-0_4")]
impl_pg!(chrono::DateTime<chrono::Utc>, "TIMESTAMP WITH TIME ZONE NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-chrono-0_4")]
impl_pg!(chrono::DateTime<chrono::Local>, "TIMESTAMP WITH TIME ZONE NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-chrono-0_4")]
impl_pg!(chrono::DateTime<chrono::FixedOffset>, "TIMESTAMP WITH TIME ZONE NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-chrono-0_4")]
impl_pg!(chrono::NaiveDate, "DATE NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-chrono-0_4")]
impl_pg!(chrono::NaiveTime, "TIME NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-eui48-0_4")]
impl_pg!(eui48::MacAddress, "MACADDR NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-geo-types-0_6")]
impl_pg!(geo_types_0_6::Point<f64>, "POINT NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-geo-types-0_6")]
impl_pg!(geo_types_0_6::Rect<f64>, "BOX NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-geo-types-0_6")]
impl_pg!(geo_types_0_6::LineString<f64>, "PATH NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-uuid-0_8")]
impl_pg!(uuid_0_8::Uuid, "UUID NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-uuid-1")]
impl_pg!(uuid_1::Uuid, "UUID NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-time-0_2")]
impl_pg!(time_0_2::PrimitiveDateTime, "TIMESTAMP NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-time-0_2")]
impl_pg!(time_0_2::OffsetDateTime, "TIMESTAMP WITH TIME ZONE NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-time-0_2")]
impl_pg!(time_0_2::Date, "DATE NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-time-0_2")]
impl_pg!(time_0_2::Time, "TIME NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-time-0_3")]
impl_pg!(time_0_3::PrimitiveDateTime, "TIMESTAMP NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-time-0_3")]
impl_pg!(time_0_3::OffsetDateTime, "TIMESTAMP WITH TIME ZONE NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-time-0_3")]
impl_pg!(time_0_3::Date, "DATE NOT NULL");

#[rustfmt::skip]
#[cfg(feature = "with-time-0_3")]
impl_pg!(time_0_3::Time, "TIME NOT NULL");
