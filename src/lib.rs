/// Contains all the necessary enums and structures to interface with the orderbook.
pub mod models;
/// Contains orderbook, store structs as well as all the core orderbook methods.
pub mod orderbook;
/// Store is a private module that contains the structure used to represent the order store.
mod store;
pub mod protobuf;