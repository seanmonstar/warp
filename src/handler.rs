use ::reply::Reply;

pub trait Handler<I> {
    type Output: Reply;

    fn handle(&self, input: I) -> Self::Output;
}

pub trait IntoHandler<I> {
    type Output: Reply;
    type Handler: Handler<I>;

    fn into_handler(self) -> Self::Handler;
}

pub struct Fn0<F>(F);

impl<F, O> Handler<()> for Fn0<F>
where
    F: Fn() -> O,
    O: Reply,
{
    type Output = O;

    fn handle(&self, _: ()) -> O {
        (self.0)()
    }
}

impl<F, O> IntoHandler<()> for F
where
    F: Fn() -> O,
    O: Reply,
{
    type Output = O;
    type Handler = Fn0<F>;

    fn into_handler(self) -> Self::Handler {
        Fn0(self)
    }
}

pub struct Fn1<F>(F);

impl<F, I, O> Handler<(I,)> for Fn1<F>
where
    F: Fn(I) -> O,
    O: Reply,
{
    type Output = O;

    fn handle(&self, input: (I,)) -> O {
        (self.0)(input.0)
    }
}

impl<F, I, O> IntoHandler<(I,)> for F
where
    F: Fn(I) -> O,
    O: Reply,
{
    type Output = O;
    type Handler = Fn1<F>;

    fn into_handler(self) -> Self::Handler {
        Fn1(self)
    }
}

pub struct Fn2<F>(F);

impl<F, I1, I2, O> Handler<(I1, I2)> for Fn2<F>
where
    F: Fn(I1, I2) -> O,
    O: Reply,
{
    type Output = O;

    fn handle(&self, input: (I1, I2)) -> O {
        (self.0)(input.0, input.1)
    }
}

impl<F, I1, I2, O> IntoHandler<(I1, I2)> for F
where
    F: Fn(I1, I2) -> O,
    O: Reply,
{
    type Output = O;
    type Handler = Fn2<F>;

    fn into_handler(self) -> Self::Handler {
        Fn2(self)
    }
}

