# flize

flize implements schemes for concurrent resource reclamation.
None of the implemented schemes requires any sort of global state.

This crate is useful if you have resources that require destruction
in a concurrent environment and you don't want to pay the price of locking.
