// Copyright (c) 2021 Kim J. Nordmo and WormieCorp.
// Licensed under the MIT license. See LICENSE.txt file in the project

//! Crate for making package managing easier.
//! The feature includes the ability to use a common set of information to build
//! packages, as well as any necessary downloading and validation of each
//! supporte package manager. Additionally a package compatible with the package
//! manager will be created based on the information given.

#![doc(
    html_playground_url = "https://play.rust-lang.org/",
    issue_tracker_base_url = "https://github.com/WormieCorp/aer/issues/"
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod parsers;
pub mod runners;

pub mod data {
    pub use aer_data::prelude::*;
}

pub mod web {
    pub use aer_web::response::{BinaryResponse, HtmlResponse, ResponseType};
    pub use aer_web::{errors, LinkElement, LinkType, WebRequest, WebResponse};
}
