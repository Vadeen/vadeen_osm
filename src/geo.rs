//! General geographic data structures.
use std::ops::{Add, Sub};

const COORD_PRECISION: f64 = 10_000_000.0;

/// Represents a coordinate containing latitude and longitude.
///
/// Coordinates are usually represented by floating point numbers, for coordinates in the osm system
/// we do not need more precision than 7 decimals as can be read in the [`o5m`] documentation.
///
/// The coordinates are represented as two i32 internally.
///
/// # Examples
/// ```
/// # use vadeen_osm::geo::Coordinate;
/// let coordinate = Coordinate::new(70.95, -8.67);
///
/// // Get decimal numbers
/// assert_eq!(coordinate.lat(), 70.95);
/// assert_eq!(coordinate.lon(), -8.67);
///
/// // Access internal i32 numbers
/// assert_eq!(coordinate.lat, 709500000);
/// assert_eq!(coordinate.lon, -86700000);
///
/// // You can also use the `Into` trait to construct coordinates.
/// let coordinate: Coordinate = (70.95, -8.67).into();
/// ```
///
/// [`O5m`]: https://wiki.openstreetmap.org/wiki/O5m#Numbers
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub struct Coordinate {
    pub lat: i32,
    pub lon: i32,
}

/// Represents coordinate boundary, i.e. min and max latitude and longitude.
///
/// # Examples
/// ```
/// # use vadeen_osm::geo::Boundary;
/// // Default creates boundaries cover the whole world
/// let bounds = Boundary::default();
///
/// assert_eq!(bounds.min.lat(), -90.0);
/// assert_eq!(bounds.min.lon(), -180.0);
/// assert_eq!(bounds.max.lat(), 90.0);
/// assert_eq!(bounds.max.lon(), 180.0);
///
/// // Inverted boundary is same as default, but with max as min and min as max.
/// // Useful when the boundary is intended to be dynamically expanded.
/// let mut bounds = Boundary::inverted();
///
/// assert_eq!(bounds.min.lat(), 90.0);
/// assert_eq!(bounds.min.lon(), 180.0);
/// assert_eq!(bounds.max.lat(), -90.0);
/// assert_eq!(bounds.max.lon(), -180.0);
///
/// bounds.expand((10.0, 20.0).into());
/// bounds.expand((30.0, 40.0).into());
///
/// assert_eq!(bounds.min.lat(), 10.0);
/// assert_eq!(bounds.min.lon(), 20.0);
/// assert_eq!(bounds.max.lat(), 30.0);
/// assert_eq!(bounds.max.lon(), 40.0);
/// ```
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Boundary {
    pub min: Coordinate,
    pub max: Coordinate,
    pub freeze: bool,
}

impl Coordinate {
    pub fn new(lat: f64, lon: f64) -> Coordinate {
        let int_lat = (lat * COORD_PRECISION) as i32;
        let int_lon = (lon * COORD_PRECISION) as i32;

        Coordinate {
            lat: int_lat,
            lon: int_lon,
        }
    }

    pub fn lat(self) -> f64 {
        self.lat as f64 / COORD_PRECISION
    }

    pub fn lon(self) -> f64 {
        self.lon as f64 / COORD_PRECISION
    }
}

impl Sub for Coordinate {
    type Output = Coordinate;

    fn sub(self, rhs: Self) -> Self::Output {
        Coordinate {
            lon: self.lon - rhs.lon,
            lat: self.lat - rhs.lat,
        }
    }
}

impl Add for Coordinate {
    type Output = Coordinate;

    fn add(self, rhs: Self) -> Self::Output {
        Coordinate {
            lon: self.lon + rhs.lon,
            lat: self.lat + rhs.lat,
        }
    }
}

impl From<(f64, f64)> for Coordinate {
    fn from((lat, lon): (f64, f64)) -> Self {
        Coordinate::new(lat, lon)
    }
}

impl Boundary {
    pub fn new<C: Into<Coordinate>>(min: C, max: C) -> Boundary {
        Boundary {
            min: min.into(),
            max: max.into(),
            freeze: false,
        }
    }

    /// Same as `default()` but inverted so min contains max and max contains min.
    /// Used when a boundary are intended to be expanded by coordinates.
    pub fn inverted() -> Self {
        Boundary {
            min: (90.0, 180.0).into(),
            max: (-90.0, -180.0).into(),
            freeze: false,
        }
    }

    /// Expand boundary if necessary to include a coordinate.
    pub fn expand(&mut self, c: Coordinate) {
        if self.freeze {
            return;
        }

        if c.lat > self.max.lat {
            self.max.lat = c.lat;
        }
        if c.lat < self.min.lat {
            self.min.lat = c.lat;
        }
        if c.lon > self.max.lon {
            self.max.lon = c.lon;
        }
        if c.lon < self.min.lon {
            self.min.lon = c.lon;
        }
    }
}

impl Default for Boundary {
    fn default() -> Self {
        Boundary {
            min: (-90.0, -180.0).into(),
            max: (90.0, 180.0).into(),
            freeze: false,
        }
    }
}
