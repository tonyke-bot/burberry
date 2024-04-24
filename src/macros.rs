#[macro_export]
macro_rules! map_boxed_executor {
    ($executor: expr, $variant: path) => {
        Box::new($crate::ExecutorMap::new($executor, |action| match action {
            $variant(value) => Some(value),
            _ => None,
        }))
    };
}

#[macro_export]
macro_rules! map_executor {
    ($executor: expr, $variant: path) => {
        $crate::map_boxed_executor!(Box::new($executor), $variant)
    };
}

#[macro_export]
macro_rules! map_boxed_collector {
    ($collector: expr, $variant: path) => {
        Box::new($crate::CollectorMap::new($collector, $variant))
    };
}

#[macro_export]
macro_rules! map_collector {
    ($collector: expr, $variant: path) => {
        $crate::map_boxed_collector!(Box::new($collector), $variant)
    };
}

#[macro_export]
macro_rules! submit_action {
    ($submitter: expr, $variant: path, $action: expr) => {
        $submitter.submit($variant($action));
    };
}
