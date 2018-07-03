#[derive(Debug)]
pub struct HCons<H, T>(pub H, pub T);

// The compiler wrongly says this isn't used...
#[allow(unused)]
pub type Cons<T> = HCons<T, ()>;

pub(crate) fn cons<T>(val: T) -> Cons<T> {
    HCons(val, ())
}

// Converts HCons (and ()) into tuples.
pub trait HList {
    type Tuple;

    fn flatten(self) -> Self::Tuple;
}

// The opposite of the HList trait, converts tuples into HCons...
pub trait Tuple {
    type HList;

    fn hlist(self) -> Self::HList;
}

// Combines HCons together.
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

impl<H, T, U> Combine<U> for HCons<H, T>
where
    T: Combine<U>,
{
    type Output = HCons<H, <T as Combine<U>>::Output>;

    #[inline]
    fn combine(self, other: U) -> Self::Output {
        HCons(self.0, self.1.combine(other))
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

impl<T1> HList for Cons<T1> {
    type Tuple = (T1,);

    #[inline]
    fn flatten(self) -> Self::Tuple {
        (self.0,)
    }
}

impl<T1, T2> HList for HCons<T1, Cons<T2>> {
    type Tuple = (T1, T2);

    #[inline]
    fn flatten(self) -> Self::Tuple {
        (self.0, (self.1).0)
    }
}

impl<T1, T2, T3> HList for HCons<T1, HCons<T2, Cons<T3>>> {
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
    type HList = Cons<T1>;
    #[inline]
    fn hlist(self) -> Self::HList {
        HCons(self.0, ())
    }
}

impl<T1, T2> Tuple for (T1, T2,) {
    type HList = HCons<T1, Cons<T2>>;
    #[inline]
    fn hlist(self) -> Self::HList {
        HCons(self.0, HCons(self.1, ()))
    }
}

impl<T1, T2, T3> Tuple for (T1, T2, T3,) {
    type HList = HCons<T1, HCons<T2, Cons<T3>>>;
    #[inline]
    fn hlist(self) -> Self::HList {
        HCons(self.0, HCons(self.1, HCons(self.2, ())))
    }
}

impl<T1, T2, T3, T4> Tuple for (T1, T2, T3, T4,) {
    type HList = HCons<T1, HCons<T2, HCons<T3, Cons<T4>>>>;
    #[inline]
    fn hlist(self) -> Self::HList {
        HCons(self.0, HCons(self.1, HCons(self.2, HCons(self.3, ()))))
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

impl<F, A1, R> Func<(A1,)> for F
where
    F: Fn(A1) -> R,
{
    type Output = R;

    #[inline]
    fn call(&self, args: (A1,)) -> Self::Output {
        (*self)(args.0)
    }
}

impl<F, A1, A2, R> Func<(A1, A2,)> for F
where
    F: Fn(A1, A2) -> R,
{
    type Output = R;

    #[inline]
    fn call(&self, args: (A1, A2,)) -> Self::Output {
        (*self)(args.0, args.1)
    }
}

impl<F, A1, A2, A3, R> Func<(A1, A2, A3,)> for F
where
    F: Fn(A1, A2, A3) -> R,
{
    type Output = R;

    #[inline]
    fn call(&self, args: (A1, A2, A3,)) -> Self::Output {
        (*self)(args.0, args.1, args.2)
    }
}
