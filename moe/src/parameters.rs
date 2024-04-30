use crate::errors::{MoeError, Result};
use crate::gaussian_mixture::GaussianMixture;
use crate::types::*;

#[allow(unused_imports)]
use egobox_gp::correlation_models::{
    AbsoluteExponentialCorr, Matern32Corr, Matern52Corr, SquaredExponentialCorr,
};
#[allow(unused_imports)]
use egobox_gp::mean_models::{ConstantMean, LinearMean, QuadraticMean};
use linfa::{Float, ParamGuard};
use linfa_clustering::GaussianMixtureModel;
use ndarray::{Array1, Array2, Array3};
use ndarray_rand::rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256Plus;

#[cfg(feature = "serializable")]
use serde::{Deserialize, Serialize};

pub use egobox_gp::{Inducings, SparseMethod, ThetaTuning};

#[derive(Clone)]
#[cfg_attr(feature = "serializable", derive(Serialize, Deserialize))]
pub enum GpType<F: Float> {
    FullGp,
    SparseGp {
        /// Used sparse method
        sparse_method: SparseMethod,
        /// Inducings
        inducings: Inducings<F>,
    },
}

/// Mixture of experts checked parameters
#[derive(Clone)]
#[cfg_attr(feature = "serializable", derive(Serialize, Deserialize))]
pub struct GpMixtureValidParams<F: Float, R: Rng + Clone> {
    /// Gp Type
    gp_type: GpType<F>,
    /// Number of clusters (i.e. number of experts)
    n_clusters: usize,
    /// [Recombination] mode
    recombination: Recombination<F>,
    /// Specification of GP regression models to be used
    regression_spec: RegressionSpec,
    /// Specification of GP correlation models to be used
    correlation_spec: CorrelationSpec,
    /// Theta hyperparameter tuning
    theta_tuning: ThetaTuning<F>,
    /// Number of PLS components, should be used when problem size
    /// is over ten variables or so.
    kpls_dim: Option<usize>,
    /// Number of GP hyperparameters optimization restarts
    n_start: usize,
    /// Gaussian Mixture model used to cluster
    gmm: Option<GaussianMixtureModel<F>>,
    /// GaussianMixture preset
    gmx: Option<GaussianMixture<F>>,
    /// Random number generator
    rng: R,
}

impl<F: Float, R: Rng + SeedableRng + Clone> Default for GpMixtureValidParams<F, R> {
    fn default() -> GpMixtureValidParams<F, R> {
        GpMixtureValidParams {
            gp_type: GpType::FullGp,
            n_clusters: 1,
            recombination: Recombination::Hard,
            regression_spec: RegressionSpec::CONSTANT,
            correlation_spec: CorrelationSpec::SQUAREDEXPONENTIAL,
            theta_tuning: ThetaTuning::default(),
            kpls_dim: None,
            n_start: 10,
            gmm: None,
            gmx: None,
            rng: R::from_entropy(),
        }
    }
}

impl<F: Float, R: Rng + Clone> GpMixtureValidParams<F, R> {
    /// The optional number of PLS components
    pub fn gp_type(&self) -> &GpType<F> {
        &self.gp_type
    }

    /// The number of clusters, hence the number of experts of the mixture.
    pub fn n_clusters(&self) -> usize {
        self.n_clusters
    }

    /// The recombination mode
    pub fn recombination(&self) -> Recombination<F> {
        self.recombination
    }

    /// The allowed GP regression models in the mixture
    pub fn regression_spec(&self) -> RegressionSpec {
        self.regression_spec
    }

    /// The allowed GP correlation models in the mixture
    pub fn correlation_spec(&self) -> CorrelationSpec {
        self.correlation_spec
    }

    /// The speified tuning of theta hyperparameter
    pub fn theta_tuning(&self) -> &ThetaTuning<F> {
        &self.theta_tuning
    }

    /// The optional number of PLS components
    pub fn kpls_dim(&self) -> Option<usize> {
        self.kpls_dim
    }

    /// The number of hypermarameters optimization restarts
    pub fn n_start(&self) -> usize {
        self.n_start
    }

    /// An optional gaussian mixture to be fitted to generate multivariate normal
    /// in turns used to cluster
    pub fn gmm(&self) -> Option<&GaussianMixtureModel<F>> {
        self.gmm.as_ref()
    }

    /// An optional multivariate normal used to cluster (take precedence over gmm)
    pub fn gmx(&self) -> Option<&GaussianMixture<F>> {
        self.gmx.as_ref()
    }

    /// The random generator
    pub fn rng(&self) -> R {
        self.rng.clone()
    }
}

/// Mixture of experts parameters
#[derive(Clone)]
#[cfg_attr(feature = "serializable", derive(Serialize, Deserialize))]
pub struct GpMixtureParams<F: Float, R: Rng + Clone>(GpMixtureValidParams<F, R>);

impl<F: Float> Default for GpMixtureParams<F, Xoshiro256Plus> {
    fn default() -> GpMixtureParams<F, Xoshiro256Plus> {
        GpMixtureParams(GpMixtureValidParams::default())
    }
}

impl<F: Float> GpMixtureParams<F, Xoshiro256Plus> {
    /// Constructor of GP parameters.
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> GpMixtureParams<F, Xoshiro256Plus> {
        Self::new_with_rng(GpType::FullGp, Xoshiro256Plus::from_entropy())
    }
}

impl<F: Float, R: Rng + SeedableRng + Clone> GpMixtureParams<F, R> {
    /// Constructor of Sgp parameters specifying randon number generator for reproducibility
    ///
    /// See [`new`](SparseGpMixtureParams::new) for default parameters.
    pub fn new_with_rng(gp_type: GpType<F>, rng: R) -> GpMixtureParams<F, R> {
        Self(GpMixtureValidParams {
            gp_type,
            n_clusters: 1,
            recombination: Recombination::Smooth(Some(F::one())),
            regression_spec: RegressionSpec::CONSTANT,
            correlation_spec: CorrelationSpec::SQUAREDEXPONENTIAL,
            theta_tuning: ThetaTuning::default(),
            kpls_dim: None,
            n_start: 10,
            gmm: None,
            gmx: None,
            rng,
        })
    }

    /// Sets the number of clusters
    pub fn gp_type(mut self, gp_type: GpType<F>) -> Self {
        self.0.gp_type = gp_type;
        self
    }

    /// Sets the number of clusters
    pub fn n_clusters(mut self, n_clusters: usize) -> Self {
        self.0.n_clusters = n_clusters;
        self
    }

    /// Sets the recombination mode
    pub fn recombination(mut self, recombination: Recombination<F>) -> Self {
        self.0.recombination = recombination;
        self
    }

    /// Sets the regression models used in the mixture.
    ///
    /// Only GP models with regression models allowed by this specification
    /// will be used in the mixture.  
    pub fn regression_spec(mut self, regression_spec: RegressionSpec) -> Self {
        self.0.regression_spec = regression_spec;
        self
    }

    /// Sets the correlation models used in the mixture.
    ///
    /// Only GP models with correlation models allowed by this specification
    /// will be used in the mixture.  
    pub fn correlation_spec(mut self, correlation_spec: CorrelationSpec) -> Self {
        self.0.correlation_spec = correlation_spec;
        self
    }

    /// Set value for theta hyper parameter.
    ///
    /// When theta is optimized, the internal optimization is started from `theta_init`.
    /// When theta is fixed, this set theta constant value.
    pub fn theta_init(mut self, theta_init: Vec<F>) -> Self {
        self.0.theta_tuning = match self.0.theta_tuning {
            ThetaTuning::Optimized { init: _, bounds } => ThetaTuning::Optimized {
                init: theta_init,
                bounds,
            },
            ThetaTuning::Fixed(_) => ThetaTuning::Fixed(theta_init),
        };
        self
    }

    /// Sets the number of componenets retained during PLS dimension reduction.
    pub fn kpls_dim(mut self, kpls_dim: Option<usize>) -> Self {
        self.0.kpls_dim = kpls_dim;
        self
    }

    /// Set theta hyper parameter search space.
    ///
    /// This function is no-op when theta tuning is fixed
    pub fn theta_bounds(mut self, theta_bounds: Vec<(F, F)>) -> Self {
        self.0.theta_tuning = match self.0.theta_tuning {
            ThetaTuning::Optimized { init, bounds: _ } => ThetaTuning::Optimized {
                init,
                bounds: theta_bounds,
            },
            ThetaTuning::Fixed(f) => ThetaTuning::Fixed(f),
        };
        self
    }

    /// Set theta hyper parameter tuning
    pub fn theta_tuning(mut self, theta_tuning: ThetaTuning<F>) -> Self {
        self.0.theta_tuning = theta_tuning;
        self
    }

    /// Sets the number of hyperparameters optimization restarts
    pub fn n_start(mut self, n_start: usize) -> Self {
        self.0.n_start = n_start;
        self
    }

    #[doc(hidden)]
    /// Sets the gaussian mixture (used to find the optimal number of clusters)
    pub fn gmm(mut self, gmm: GaussianMixtureModel<F>) -> Self {
        self.0.gmm = Some(gmm);
        self
    }

    #[doc(hidden)]
    /// Sets the gaussian mixture (used to find the optimal number of clusters)
    /// Warning: no consistency check is done on the given initialization data
    /// *Panic* if multivariate normal init data not sound
    pub fn gmx(mut self, weights: Array1<F>, means: Array2<F>, covariances: Array3<F>) -> Self {
        self.0.gmx = Some(GaussianMixture::new(weights, means, covariances).unwrap());
        self
    }

    /// Sets the random number generator for reproducibility
    pub fn with_rng<R2: Rng + Clone>(self, rng: R2) -> GpMixtureParams<F, R2> {
        GpMixtureParams(GpMixtureValidParams {
            gp_type: self.0.gp_type().clone(),
            n_clusters: self.0.n_clusters(),
            recombination: self.0.recombination(),
            regression_spec: self.0.regression_spec(),
            correlation_spec: self.0.correlation_spec(),
            theta_tuning: self.0.theta_tuning().clone(),
            kpls_dim: None,
            n_start: self.0.n_start(),
            gmm: self.0.gmm().cloned(),
            gmx: self.0.gmx().cloned(),
            rng,
        })
    }
}

impl<F: Float, R: Rng + Clone> ParamGuard for GpMixtureParams<F, R> {
    type Checked = GpMixtureValidParams<F, R>;
    type Error = MoeError;

    fn check_ref(&self) -> Result<&Self::Checked> {
        if let Some(d) = self.0.kpls_dim {
            if d == 0 {
                return Err(MoeError::InvalidValueError(
                    "`kpls_dim` canot be 0!".to_string(),
                ));
            }
        }
        Ok(&self.0)
    }

    fn check(self) -> Result<Self::Checked> {
        self.check_ref()?;
        Ok(self.0)
    }
}

impl<F: Float, R: Rng + Clone> From<GpMixtureValidParams<F, R>> for GpMixtureParams<F, R> {
    fn from(item: GpMixtureValidParams<F, R>) -> Self {
        GpMixtureParams(item)
    }
}
