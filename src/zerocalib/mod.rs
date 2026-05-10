//! ZeroCalib Riemannian Classifier
//!
//! ZeroCalib eliminates per-session calibration through transfer learning.
//!
//! Pipeline:
//! 1. Universal MDM classifier (pre-trained on 127 subjects)
//! 2. Euclidean alignment (domain shift reduction)
//! 3. Online Riemannian mean update (geodesic gradient descent)

pub mod riemannian;
pub mod mdm;
pub mod alignment;

pub use mdm::MdmClassifier;
pub use alignment::EuclideanAlignment;
