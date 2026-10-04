//! Gunmetal's HTTP foundation: the route table, the request pipeline,
//! security headers, body limits and the problem renderer
//! (web-and-api-security.md, "The route table" and "Request pipeline").
//!
//! Every route is a [`route::RouteSpec`] in one [`table::Table`], and the
//! router is built only from that table ([`pipeline::router`]), so the
//! allow-list, role-matrix and header tests can enumerate every route the
//! server answers.

pub mod browser;
pub mod call;
pub mod client;
pub mod credential;
pub mod decode;
pub mod headers;
pub mod host;
pub mod paging;
pub mod pipeline;
pub mod problem;
mod query;
pub mod request;
pub mod route;
pub mod table;
