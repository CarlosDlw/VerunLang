use crate::ast::nodes::*;

pub trait CodeTarget {
    fn name(&self) -> &str;
    fn file_extension(&self) -> &str;
    fn generate(&self, program: &Program) -> String;
}

pub fn generate_for_target(program: &Program, target: &dyn CodeTarget) -> String {
    target.generate(program)
}
