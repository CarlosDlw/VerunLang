use z3::ast::Dynamic;
use z3::{Config, Context, SatResult, Solver as Z3Solver};

pub struct Solver {
    config: Config,
}

pub struct SolverSession<'ctx> {
    pub ctx: &'ctx Context,
    pub solver: Z3Solver<'ctx>,
}

#[derive(Debug, Clone)]
pub enum CheckResult {
    Verified,
    Failed {
        counterexample: Vec<(String, String)>,
    },
    Unknown {
        reason: String,
    },
}

impl Solver {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.set_param_value("timeout", "30000");
        Self { config }
    }

    pub fn create_context(&self) -> Context {
        Context::new(&self.config)
    }
}

impl<'ctx> SolverSession<'ctx> {
    pub fn new(ctx: &'ctx Context) -> Self {
        Self {
            ctx,
            solver: Z3Solver::new(ctx),
        }
    }

    pub fn assert(&self, constraint: &z3::ast::Bool<'ctx>) {
        self.solver.assert(constraint);
    }

    pub fn check(&self) -> CheckResult {
        match self.solver.check() {
            SatResult::Unsat => CheckResult::Verified,
            SatResult::Sat => CheckResult::Failed {
                counterexample: Vec::new(),
            },
            SatResult::Unknown => CheckResult::Unknown {
                reason: self
                    .solver
                    .get_reason_unknown()
                    .unwrap_or_else(|| "unknown".to_string()),
            },
        }
    }

    pub fn extract_counterexample(
        &self,
        vars: &[(String, Dynamic<'ctx>)],
    ) -> Vec<(String, String)> {
        let mut values = Vec::new();
        if let Some(model) = self.solver.get_model() {
            for (name, var) in vars {
                if let Some(val) = model.eval(var, true) {
                    values.push((name.clone(), val.to_string()));
                }
            }
        }
        values
    }

    pub fn push(&self) {
        self.solver.push();
    }

    pub fn pop(&self) {
        self.solver.pop(1);
    }

    pub fn reset(&self) {
        self.solver.reset();
    }
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}
