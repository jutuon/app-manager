use error_stack::{Context, IntoReport, Report, Result, ResultExt};
use tokio::sync::oneshot;

/// Sender only used for quit request message sending.
pub type QuitSender = oneshot::Sender<()>;

/// Receiver only used for quit request message receiving.
pub type QuitReceiver = oneshot::Receiver<()>;

pub trait IntoReportExt: IntoReport {
    #[track_caller]
    fn into_error<C: Context>(self, context: C) -> Result<<Self as IntoReport>::Ok, C> {
        self.into_report().change_context(context)
    }

    #[track_caller]
    fn into_error_with_info<
        C: Context,
        I: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    >(
        self,
        context: C,
        info: I,
    ) -> Result<<Self as IntoReport>::Ok, C> {
        self.into_report()
            .change_context(context)
            .attach_printable(info)
    }

    #[track_caller]
    fn into_error_with_info_lazy<
        C: Context,
        F: FnOnce() -> I,
        I: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    >(
        self,
        context: C,
        info: F,
    ) -> Result<<Self as IntoReport>::Ok, C> {
        self.into_report()
            .change_context(context)
            .attach_printable_lazy(info)
    }
}

impl<T: IntoReport> IntoReportExt for T {}

pub trait ErrorResultExt: ResultExt + Sized {
    #[track_caller]
    fn change_context_with_info<
        C: Context,
        I: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    >(
        self,
        context: C,
        info: I,
    ) -> Result<<Self as ResultExt>::Ok, C> {
        self.change_context(context).attach_printable(info)
    }

    #[track_caller]
    fn change_context_with_info_lazy<
        C: Context,
        F: FnOnce() -> I,
        I: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    >(
        self,
        context: C,
        info: F,
    ) -> Result<<Self as ResultExt>::Ok, C> {
        self.change_context(context).attach_printable_lazy(info)
    }
}

impl<T: ResultExt + Sized> ErrorResultExt for T {}

pub trait ErrorConversion: ResultExt + Sized {
    type Err: Context;
    const ERROR: Self::Err;

    /// Change error context and add additional info about error.
    #[track_caller]
    fn with_info<I: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static>(
        self,
        info: I,
    ) -> Result<<Self as ResultExt>::Ok, Self::Err> {
        self.change_context_with_info(Self::ERROR, info)
    }

    /// Change error context and add additional info about error. Sets
    /// additional info about error lazily.
    #[track_caller]
    fn with_info_lazy<
        F: FnOnce() -> I,
        I: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    >(
        self,
        info: F,
    ) -> Result<<Self as ResultExt>::Ok, Self::Err> {
        self.change_context_with_info_lazy(Self::ERROR, info)
    }
}

pub type ErrorContainer<E> = Option<Report<E>>;

pub trait AppendErr: Sized {
    type E: Context;

    fn append(&mut self, e: Report<Self::E>);
    fn into_result(self) -> Result<(), Self::E>;
}

pub trait AppendErrorTo<Err>: Sized {
    fn append_to_and_ignore(self, container: &mut ErrorContainer<Err>);
    fn append_to_and_return_container(self, container: &mut ErrorContainer<Err>)
        -> Result<(), Err>;
}

impl<Ok, Err: Context> AppendErrorTo<Err> for Result<Ok, Err>
where
    ErrorContainer<Err>: AppendErr<E = Err>,
{
    fn append_to_and_ignore(self, container: &mut ErrorContainer<Err>) {
        if let Err(e) = self {
            container.append(e)
        }
    }

    fn append_to_and_return_container(
        self,
        container: &mut ErrorContainer<Err>,
    ) -> Result<(), Err> {
        if let Err(e) = self {
            container.append(e);
            container.take().into_result()
        } else {
            Ok(())
        }
    }
}
