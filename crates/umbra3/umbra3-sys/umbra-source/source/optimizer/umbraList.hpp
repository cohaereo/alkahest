// Copyright (c) 2010-2012 Umbra Software Ltd.
// All rights reserved. www.umbrasoftware.com

#ifndef UMBRALIST_HPP
#define UMBRALIST_HPP

namespace Umbra
{ 

//------------------------------------------------------------------------

template <class T> 
class List
{
public: 

	UMBRA_INLINE List(void)					
    :   m_next    (NULL)
    ,   m_prev    (NULL) 
    {
    } 

	UMBRA_INLINE List(const T& V)			
    :   m_value   (V)
    ,   m_next    (NULL)
    ,   m_prev    (NULL) 
    {
    } 

	UMBRA_INLINE const T& Get(void) const			
    { 
        return m_value; 
    } 

	UMBRA_INLINE T&	Get(void)					
    { 
        return m_value; 
    } 

	T		    m_value; 
	List<T>*	m_next; 
	List<T>*	m_prev; 
}; 

//------------------------------------------------------------------------

template <class T> List<T>* link(List<T>* A, List<T>* B)
{ 
	UMBRA_ASSERT(B == NULL || (B && B->m_prev == NULL)); 
	UMBRA_ASSERT(A == NULL || (A && A->m_prev == NULL)); 

	if (!A)
		return B; 

	List<T>* tail = A; 

	while (tail->m_next)
		tail = tail->m_next; 

	tail->m_next = B; 
	if (B)
		B->m_prev = tail; 

	return A; 
} 

//------------------------------------------------------------------------

template <class T> UMBRA_INLINE List<T>* unlink(List<T>* root, List<T>* node)					
{ 
	UMBRA_ASSERT(node && root); 

	List<T>* m_next = node->m_next; 

	if (node->m_prev) node->m_prev->m_next = node->m_next; 
	if (node->m_next) node->m_next->m_prev = node->m_prev; 

	node->m_next = node->m_prev = NULL; 

	if (node == root)
		return m_next; 

	return root; 
} 

//------------------------------------------------------------------------

template <class T> int size(List<T>* node)
{
	int size = 0; 
	while (node)
	{
		size++; 
		node = node->m_next;
	}   
	return size; 
}

//------------------------------------------------------------------------

template <class T> List<T>* find(List<T>* node, const T& m_value)
{
	while (node)
	{
		if (node->Get() == m_value)
			return node; 
		node = node->m_next; 
	} 

	return NULL; 
} 

//------------------------------------------------------------------------

} // namespace Umbra

#endif // UMBRALIST_HPP 
