use crate::lint::Analyzer;

mod no_allow_attr;
mod no_db_mut_in_test;
mod no_dead_guard;
mod no_env_branch;
mod no_env_var;
mod no_error_wrap_banality;
mod no_inline_comment;
mod no_panic_src;
mod no_redundant_if;
mod no_robot_doc;
mod no_silent_fallback;
mod no_time_now;
mod no_todo;
mod no_type_only_assert;

pub fn all() -> Vec<Box<dyn Analyzer>> {
    vec![
        Box::new(no_inline_comment::NoInlineComment),
        Box::new(no_allow_attr::NoAllowAttr),
        Box::new(no_env_var::NoEnvVar),
        Box::new(no_env_branch::NoEnvBranch),
        Box::new(no_panic_src::NoPanicSrc),
        Box::new(no_time_now::NoTimeNow),
        Box::new(no_type_only_assert::NoTypeOnlyAssert),
        Box::new(no_db_mut_in_test::NoDbMutInTest),
        Box::new(no_robot_doc::NoRobotDoc),
        Box::new(no_error_wrap_banality::NoErrorWrapBanality),
        Box::new(no_todo::NoTodo),
        Box::new(no_redundant_if::NoRedundantIf),
        Box::new(no_dead_guard::NoDeadGuard),
        Box::new(no_silent_fallback::NoSilentFallback),
    ]
}
