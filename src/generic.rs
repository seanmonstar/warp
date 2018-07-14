#[derive(Debug)]
pub struct Product<H, T>(pub H, pub T);

pub type One<T> = Product<T, ()>;

#[inline]
pub(crate) fn one<T>(val: T) -> One<T> {
    Product(val, ())
}

// Converts Product (and ()) into tuples.
pub trait HList {
    type Tuple;

    fn flatten(self) -> Self::Tuple;
}

// The opposite of the HList trait, converts tuples into Product...
pub trait Tuple {
    type HList;

    fn hlist(self) -> Self::HList;
}

// Combines Product together.
pub trait Combine<T> {
    type Output;

    fn combine(self, other: T) -> Self::Output;
}

pub trait Func<Args> {
    type Output;

    fn call(&self, args: Args) -> Self::Output;
}

// ===== impl Combine =====

impl<T> Combine<T> for () {
    type Output = T;
    #[inline]
    fn combine(self, other: T) -> Self::Output {
        other
    }
}

impl<H, T, U> Combine<U> for Product<H, T>
where
    T: Combine<U>,
{
    type Output = Product<H, <T as Combine<U>>::Output>;

    #[inline]
    fn combine(self, other: U) -> Self::Output {
        Product(self.0, self.1.combine(other))
    }
}

// ===== impl HList =====

impl HList for () {
    type Tuple = ();
    #[inline]
    fn flatten(self) -> Self::Tuple {
        ()
    }
}

impl<T1> HList for One<T1> {
    type Tuple = (T1,);

    #[inline]
    fn flatten(self) -> Self::Tuple {
        (self.0,)
    }
}

impl<T1, T2> HList for Product<T1, One<T2>> {
    type Tuple = (T1, T2);

    #[inline]
    fn flatten(self) -> Self::Tuple {
        (self.0, (self.1).0)
    }
}

impl<T1, T2, T3> HList for Product<T1, Product<T2, One<T3>>> {
    type Tuple = (T1, T2, T3);

    #[inline]
    fn flatten(self) -> Self::Tuple {
        (self.0, (self.1).0, (((self.1).1).0))
    }
}

// ===== impl Tuple =====

impl Tuple for () {
    type HList = ();
    #[inline]
    fn hlist(self) -> Self::HList {
        ()
    }
}

impl<T1> Tuple for (T1,) {
    type HList = One<T1>;
    #[inline]
    fn hlist(self) -> Self::HList {
        Product(self.0, ())
    }
}

impl<T1, T2> Tuple for (T1, T2,) {
    type HList = Product<T1, One<T2>>;
    #[inline]
    fn hlist(self) -> Self::HList {
        Product(self.0, Product(self.1, ()))
    }
}

impl<T1, T2, T3> Tuple for (T1, T2, T3,) {
    type HList = Product<T1, Product<T2, One<T3>>>;
    #[inline]
    fn hlist(self) -> Self::HList {
        Product(self.0, Product(self.1, Product(self.2, ())))
    }
}

impl<T1, T2, T3, T4> Tuple for (T1, T2, T3, T4,) {
    type HList = Product<T1, Product<T2, Product<T3, One<T4>>>>;
    #[inline]
    fn hlist(self) -> Self::HList {
        Product(self.0, Product(self.1, Product(self.2, Product(self.3, ()))))
    }
}

// ===== impl Func =====

impl<F, R> Func<()> for F
where
    F: Fn() -> R,
{
    type Output = R;

    #[inline]
    fn call(&self, _args: ()) -> Self::Output {
        (*self)()
    }
}

impl<F, A1, R> Func<One<A1>> for F
where
    F: Fn(A1) -> R,
{
    type Output = R;

    #[inline]
    fn call(&self, args: One<A1>) -> Self::Output {
        (*self)(args.0)
    }
}

impl<F, A1, A2, R> Func<Product<A1, One<A2>>> for F
where
    F: Fn(A1, A2) -> R,
{
    type Output = R;

    #[inline]
    fn call(&self, args: Product<A1, One<A2>>) -> Self::Output {
        (*self)(args.0, (args.1).0)
    }
}

impl<F, A1, A2, A3, R> Func<Product<A1, Product<A2, One<A3>>>> for F
where
    F: Fn(A1, A2, A3) -> R,
{
    type Output = R;

    #[inline]
    fn call(&self, args: Product<A1, Product<A2, One<A3>>>) -> Self::Output {
        (*self)(args.0, (args.1).0, ((args.1).1).0)
    }
}

