export interface InvoiceItem {
  id?: number;
  inventory_id: number;
  quantity: number;
  rate: number;
  discount: number; // Item-level discount amount (not percentage)
  total: number;
}

/**
 * Calculates the total for a single line item.
 * @param quantity Quantity of the item
 * @param rate Price per unit
 * @param discount Item-level discount (fixed amount)
 * @returns The calculated total for the item
 */
export function calculateItemTotal(quantity: number, rate: number, discount: number = 0): number {
  const subtotal = quantity * rate;
  return Math.max(0, subtotal - discount);
}

/**
 * Calculates the subtotal for all items in the invoice.
 * @param items Array of invoice items
 * @returns Sum of all item totals
 */
export function calculateSubtotal(items: InvoiceItem[]): number {
  return items.reduce((sum, item) => sum + item.total, 0);
}

/**
 * Calculates the final grand total, applying global discount and optional rounding.
 * @param subtotal The sum of all item totals
 * @param globalDiscount The invoice-level discount (fixed amount)
 * @param isCashSale Whether this is a cash sale (triggers rounding)
 * @returns The final total, rounded to the nearest whole number if cash sale
 */
export function calculateGrandTotal(subtotal: number, globalDiscount: number = 0, isCashSale: boolean = false): number {
  const total = Math.max(0, subtotal - globalDiscount);
  return isCashSale ? Math.round(total) : total;
}

/**
 * Checks if the current sale price is below the cost price.
 * @param salePrice The current rate entered
 * @param costPrice The actual cost price of the item
 * @returns True if selling below cost
 */
export function isSellingBelowCost(salePrice: number, costPrice: number): boolean {
  return salePrice < costPrice;
}
