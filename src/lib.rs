impl<'a,E> Deref for SliceInitTransaction<'a,E>{
	fn deref(&self)->&Self::Target{
		unsafe{		// inner has layout of E and is initialized up to len
			let ptr=self.inner.as_ptr() as *const E;
			let len=self.len;

			slice::from_raw_parts(ptr,len)
		}
	}
	type Target=[E];
}
impl<'a,E> DerefMut for SliceInitTransaction<'a,E>{
	fn deref_mut(&mut self)->&mut Self::Target{
		unsafe{		// inner has layout of E and is initialized up to len
			let ptr=self.inner.as_mut_ptr() as *mut E;
			let len=self.len;

			slice::from_raw_parts_mut(ptr,len)
		}
	}
}
impl<'a,E> Drop for SliceInitTransaction<'a,E>{
	fn drop(&mut self){self.truncate(0)}
}
impl<'a,E> SliceInitTransaction<'a,E>{
	/// get the capacity of the inner slice
	pub fn cap(&self)->usize{self.inner.len()}
	/// commit the transation, leaving items up to self.len initialized
	pub fn commit(self)->&'a mut [E]{
		unsafe{		// inner has layout of E and is initialized up to len
			let ptr=self.inner.as_mut_ptr() as *mut E;
			let len=self.len;
			// forget so the items remain initialized
			mem::forget(self);
			slice::from_raw_parts_mut(ptr,len)
		}
	}
	/// access the inner slice. for unsafe purposes, it's guaranteed initialized up to self.len
	pub fn inner(&self)->&[MaybeUninit<E>]{&self.inner}
	/// access the inner slice. for unsafe purposes, it's guaranteed initialized up to self.len
	pub fn inner_mut(&mut self)->&mut [MaybeUninit<E>]{&mut self.inner}
	/// create a new slice initialization transaction
	pub fn new(inner:&'a mut [MaybeUninit<E>])->Self{
		Self{inner,len:0}
	}
	/// push a component into the transaction. components are written into the slice in the order pushed. returns Err(item) if full
	pub fn push(&mut self,item:E)->Result<(),E>{
		if self.inner.len()==self.len{return Err(item)}

		self.inner[self.len]=MaybeUninit::new(item);
		self.len+=1;

		Ok(())
	}
	/// create a partially initialized slice initialization transaction. inner must be initialized up to len
	pub unsafe fn partial(inner:&'a mut [MaybeUninit<E>],len:usize)->Self{
		Self{inner,len}
	}
	/// set the length. the inner slice must be initialized up to len. for non Copy components, reducing the len may lead to a memory leak if they are overwritten later
	pub unsafe fn set_len(&mut self,len:usize){self.len=len}
	/// set the length to the minimum of len and self.len. drops any components at indices between len and self.len if the length is reduced
	pub fn truncate(&mut self,len:usize){
		for n in len..self.len{		// initialization is maintained up to len
			unsafe{self.inner[n].assume_init_drop()}
		}
		self.len=len.min(self.len);
	}
}

impl<E,const N:usize> SliceLikeInit for [E;N]{
	unsafe fn accept<E2>(self,len:usize)->Self::WithComponent<E2> where MaybeUninit<E2>:IsEqual<Self::E>,Self:Sized,Self::WithComponent<E2>:Sized{
		assert_eq!(len,self.len());
		unsafe{		// len of initialization receipt match, len is required by precondition to still be accurate, and E is required by precondition to be transmutable to E2 when initialized
			mem::transmute_copy(&MaybeUninit::new(self))
		}
	}
}
impl<E,const N:usize> SliceLike for [E;N]{
	fn slice(&self)->&[E]{self.as_ref()}
	fn mut_slice(&mut self)->&mut [E]{self.as_mut()}
	type E=E;
	type WithComponent<E2>=[E2;N];
}
impl<E> SliceLikeInit for [E]{}
impl<E> SliceLikeInit for Box<[E]>{
	unsafe fn accept<E2>(mut self,len:usize)->Self::WithComponent<E2> where MaybeUninit<E2>:IsEqual<Self::E>,Self:Sized,Self::WithComponent<E2>:Sized{
		assert_eq!(len,self.len());

		unsafe{		// len of initialization receipt match, len is required by precondition to still be accurate, and E is required by precondition to be transmutable to E2 when initialized
			let ptr=self.as_mut_ptr() as *mut E2;
			let len=self.len();

			mem::forget(self);
			Box::from_raw(slice::from_raw_parts_mut(ptr,len))
		}
	}
}
impl<E> SliceLikeInit for Vec<E>{
	unsafe fn accept<E2>(mut self,len:usize)->Self::WithComponent<E2> where MaybeUninit<E2>:IsEqual<Self::E>,Self:Sized,Self::WithComponent<E2>:Sized{
		assert!(len>=self.len());
		assert!(len<=self.capacity());

		unsafe{		// len of initialization receipt match, len is required by precondition to still be accurate, and E is required by precondition to be transmutable to E2 when initialized
			let ptr=self.as_mut_ptr() as *mut E2;
			let cap=self.capacity();

			mem::forget(self);
			Vec::from_raw_parts(ptr,len,cap)
		}
	}
	/// begin a slice init transaction. Being a transaction, it drops components up to len on drop, leaving the inner slice uninitialized, giving a reasonable approximation of having no effect if uncommitted. However, data that might have preexisted is not restored
	fn begin_slice_init_transaction<E2>(&mut self)->SliceInitTransaction<'_,E2> where MaybeUninit<E2>:IsEqual<Self::E>{
		let ptr=self.as_mut_ptr();
		let cap=self.capacity();

		unsafe{
			let uuninit=slice::from_raw_parts_mut(ptr as *mut MaybeUninit<E2>,cap);
			SliceInitTransaction::new(uuninit)
		}
	}
}
impl<E> SliceLike for [E]{
	fn slice(&self)->&[E]{self}
	fn mut_slice(&mut self)->&mut [E]{self}
	type E=E;
	type WithComponent<E2>=[E2];
}
impl<E> SliceLike for Box<[E]>{
	fn slice(&self)->&[E]{&*self}
	fn mut_slice(&mut self)->&mut [E]{&mut *self}
	type E=E;
	type WithComponent<E2>=Box<[E2]>;
}
impl<E> SliceLike for Vec<E>{
	fn slice(&self)->&[E]{&*self}
	fn mut_slice(&mut self)->&mut [E]{&mut *self}
	type E=E;
	type WithComponent<E2>=Vec<E2>;
}

#[cfg(test)]
mod test{
	#[test]
	fn extra_vec_len(){
		let mut x:Vec<MaybeUninit<u32>>=vec![MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit()];
		x.reserve(1);

		let x:Vec<u32>=x.init_values_with_slice_transaction(|transaction|for n in 1..7{transaction.push(n).unwrap()});

		assert_eq!(x,[1,2,3,4,5,6]);
	}
	#[test]
	fn init_array(){
		let x:[MaybeUninit<u32>;5]=[MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit()];
		let x:[u32;5]=x.init_values_with_slice_transaction(|transaction|for n in 1..6{transaction.push(n).unwrap()});

		assert_eq!(x,[1,2,3,4,5]);
	}
	#[test]
	fn init_slice(){
		let mut x:[MaybeUninit<u32>;5]=[MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit()];
		let mut initx=SliceInitTransaction::new(&mut x);

		assert_eq!(&*initx,[].as_slice());
		initx.push(10).unwrap();

		assert_eq!(&*initx,[10].as_slice());
		initx.push(10).unwrap();

		assert_eq!(&*initx,[10,10].as_slice());
		initx.push(9).unwrap();

		assert_eq!(&*initx,[10,10, 9].as_slice());
		initx.push(2).unwrap();

		assert_eq!(initx.commit(),[10,10,9,2].as_slice());
	}
	#[test]
	fn init_vec(){
		let x:Vec<MaybeUninit<u32>>=vec![MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit()];
		let x:Vec<u32>=x.init_values_with_slice_transaction(|transaction|for n in 1..6{transaction.push(n).unwrap()});

		assert_eq!(x,[1,2,3,4,5]);
	}
	#[test]
	#[should_panic]
	fn insufficient_vec_len(){
		let x:Vec<MaybeUninit<u32>>=vec![MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit(),MaybeUninit::uninit()];
		let x:Vec<u32>=x.init_values_with_slice_transaction(|transaction|for n in 1..5{transaction.push(n).unwrap()});

		dbg!(x);
	}
	#[test]
	#[should_panic]
	fn overfill_slice(){
		let mut x:[MaybeUninit<u32>;2]=[MaybeUninit::uninit(),MaybeUninit::uninit()];
		let mut initx=SliceInitTransaction::new(&mut x);

		assert_eq!(&*initx,[].as_slice());
		initx.push(10).unwrap();

		assert_eq!(&*initx,[10].as_slice());
		initx.push(10).unwrap();

		assert_eq!(&*initx,[10,10].as_slice());
		initx.push(9).unwrap();

		assert_eq!(&*initx,[10,10, 9].as_slice());
		initx.push(2).unwrap();

		assert_eq!(initx.commit(),[10,10,9,2].as_slice());
	}

	use super::*;
}

/// transaction for slice initialization, add items with push. Being a transaction, it drops components up to len on drop, leaving the inner slice uninitialized, giving a reasonable approximation of having no effect if uncommitted. However, data that might have preexisted is not restored
pub struct SliceInitTransaction<'a,E>{inner:&'a mut [MaybeUninit<E>],len:usize}

/// a collection such that if its items are MaybeUninit, it would have a data initialization process reducible to the process for initializing a slice of MaybeUninit
pub trait SliceLikeInit:SliceLike{
	/// assume data initialized up to len. The items from ptr=self.mut_slice().as_ptr() to ptr+len must still be initialized, and Self::E must be MaybeUninit and transmutable to E2 when initialized. may panic on detection that the initialization is inappropriate or incomplete, such as an array getting insufficiently many items
	unsafe fn accept<E2>(self,len:usize)->Self::WithComponent<E2> where MaybeUninit<E2>:IsEqual<Self::E>,Self:Sized,Self::WithComponent<E2>:Sized;
	/// begin a slice init transaction. Being a transaction, it drops components up to len on drop, leaving the inner slice uninitialized, giving a reasonable approximation of having no effect if uncommitted. However, data that might have preexisted is not restored
	fn begin_slice_init_transaction<E2>(&mut self)->SliceInitTransaction<'_,E2> where MaybeUninit<E2>:IsEqual<Self::E>{SliceInitTransaction::new(self.cast_mut_slice::<MaybeUninit<E2>>())}
	/// run a closure to initialize the values in a collection of MaybeUninit<E> safely by pushing into a SliceInitTransaction, then convert to a collection of E
	fn init_values_with_slice_transaction<E2,F:for<'a>FnOnce(&mut SliceInitTransaction<'a,E2>)>(mut self,init:F)->Self::WithComponent<E2> where MaybeUninit<E2>:IsEqual<Self::E>,Self:Sized,Self::WithComponent<E2>:Sized{
		unsafe{
			let mut transaction=self.begin_slice_init_transaction();
			init(&mut transaction);

			let receipt=transaction.commit().len();
			self.accept(receipt)
		}
	}
}
/// describes potentially mutable collections that are laid out like generic slices
pub trait SliceLike{
	/// cast using type equals
	fn cast_mut_slice<E2:IsEqual<Self::E>>(&mut self)->&mut [E2]{
		unsafe{mem::transmute(self.mut_slice())} // the type is equals
	}
	/// reference as a slice
	fn slice(&self)->&[Self::E];
	/// reference as a slice
	fn mut_slice(&mut self)->&mut [Self::E];
	/// the component type
	type E;
	/// type of the collection but with a different component type
	type WithComponent<E2>:?Sized+SliceLike<E=E2,WithComponent<E2>=Self::WithComponent<E2>>+SliceLike<E=E2,WithComponent<Self::E>=Self>;
}

use std::{
	mem::{MaybeUninit,self},ops::{Deref,DerefMut},slice
};
use type_equalities::IsEqual;
