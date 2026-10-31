import type { Category } from '@/shared/api'

export function categoryTone(category: Category | null): string {
  return `category-tone category-tone--${category ?? 'none'}`
}

export function categoryFill(category: Category): string {
  return `category-fill category-fill--${category}`
}
