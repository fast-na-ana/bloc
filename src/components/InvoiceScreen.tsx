import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Trash2, Search, Receipt, Plus, Check } from "lucide-react";
import {
  InvoiceItem,
  calculateItemTotal,
  calculateSubtotal,
  calculateGrandTotal,
  isSellingBelowCost,
} from "@/lib/invoice-calc";
import { cn } from "@/lib/utils";

interface Account {
  id: number;
  name: string;
  account_type: string;
  credit_limit: number;
}

interface Inventory {
  id: number;
  name: string;
  stock_level: number;
  cost_price: number;
  sale_price: number;
}

const containerVars = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: { staggerChildren: 0.05 }
  }
} as any;

const itemVars = {
  hidden: { opacity: 0, y: 15 },
  show: { opacity: 1, y: 0, transition: { duration: 0.4, ease: "easeOut" } }
} as any;

export function InvoiceScreen() {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [inventoryList, setInventoryList] = useState<Inventory[]>([]);
  
  const [invoiceType, setInvoiceType] = useState<string>("sale");
  const [accountId, setAccountId] = useState<string>("");
  const [invoiceDate, setInvoiceDate] = useState<string>(
    new Date().toISOString().split("T")[0]
  );
  
  const [items, setItems] = useState<InvoiceItem[]>([]);
  const [globalDiscount, setGlobalDiscount] = useState<number>(0);
  
  const [openCommand, setOpenCommand] = useState<number | null>(null);

  useEffect(() => {
    async function loadData() {
      try {
        const acc = await invoke<Account[]>("get_accounts");
        const inv = await invoke<Inventory[]>("get_inventory");
        setAccounts(acc);
        setInventoryList(inv);
      } catch (err) {
        console.error("Failed to load initial data", err);
      }
    }
    loadData();
  }, []);

  const handleAddItem = () => {
    setItems([
      ...items,
      { inventory_id: 0, quantity: 1, rate: 0, discount: 0, total: 0 },
    ]);
  };

  const handleRemoveItem = (index: number) => {
    const newItems = [...items];
    newItems.splice(index, 1);
    setItems(newItems);
  };

  const updateItem = (index: number, field: keyof InvoiceItem, value: any) => {
    const newItems = [...items];
    const item = { ...newItems[index], [field]: value };
    
    if (field === "inventory_id") {
      const selectedInv = inventoryList.find((i) => i.id === value);
      if (selectedInv) {
        item.rate =
          invoiceType === "purchase" || invoiceType === "purchase_return"
            ? selectedInv.cost_price
            : selectedInv.sale_price;
      }
    }

    item.total = calculateItemTotal(item.quantity, item.rate, item.discount);
    newItems[index] = item;
    setItems(newItems);
  };

  const handleSave = async () => {
    if (!accountId || items.length === 0) return;
    
    try {
      const payload = {
        invoice_type: invoiceType,
        account_id: parseInt(accountId),
        date: invoiceDate,
        total: grandTotal,
        discount: globalDiscount,
        items: items,
      };
      
      const res = await invoke<string>("create_invoice", { payload });
      alert(res);
      setItems([]);
      setGlobalDiscount(0);
      setAccountId("");
    } catch (err) {
      alert("Error creating invoice: " + err);
    }
  };

  const subtotal = calculateSubtotal(items);
  const selectedAccount = accounts.find((a) => a.id === parseInt(accountId));
  const isCashSale =
    selectedAccount?.name.toLowerCase().includes("cash") || false;
    
  const grandTotal = calculateGrandTotal(subtotal, globalDiscount, isCashSale);

  return (
    <motion.div 
      variants={containerVars}
      initial="hidden"
      animate="show"
      className="grid grid-cols-12 gap-6 h-full auto-rows-max"
    >
      {/* HEADER BAR */}
      <motion.div variants={itemVars} className="col-span-12 glass-widget p-6 hover-lift flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-2xl bg-blue-50 border border-blue-100 flex items-center justify-center shadow-xs">
            <Receipt className="text-blue-600 h-5 w-5" />
          </div>
          <div>
            <h1 className="text-2xl font-extrabold text-slate-900 leading-tight">
              Sales & Purchases Billing
            </h1>
            <p className="text-slate-500 text-sm font-medium">Create invoices, returns, and automatic ledger entries</p>
          </div>
        </div>
        <Button 
          onClick={handleSave} 
          size="lg"
          className="bg-blue-600 hover:bg-blue-700 text-white font-semibold shadow-md shadow-blue-600/20 transition-all cursor-pointer"
        >
          <Check className="h-4 w-4 mr-2" /> Save & Auto-Merge
        </Button>
      </motion.div>

      {/* FORM HEADER CONTROLS */}
      <motion.div variants={itemVars} className="col-span-12 glass-widget p-6 hover-lift">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <div className="space-y-2">
            <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Invoice Type</label>
            <Select value={invoiceType} onValueChange={setInvoiceType}>
              <SelectTrigger className="bg-white/80 border-slate-200 focus:ring-blue-500 text-slate-900 font-semibold">
                <SelectValue placeholder="Select type" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="sale">Sale Invoice</SelectItem>
                <SelectItem value="purchase">Purchase Invoice</SelectItem>
                <SelectItem value="sale_return">Sale Return</SelectItem>
                <SelectItem value="purchase_return">Purchase Return</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Account Party</label>
            <Select value={accountId} onValueChange={setAccountId}>
              <SelectTrigger className="bg-white/80 border-slate-200 focus:ring-blue-500 text-slate-900 font-semibold">
                <SelectValue placeholder="Select Account" />
              </SelectTrigger>
              <SelectContent>
                {accounts.map((acc) => (
                  <SelectItem key={acc.id} value={acc.id.toString()}>
                    {acc.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Document Date</label>
            <Input
              type="date"
              value={invoiceDate}
              onChange={(e) => setInvoiceDate(e.target.value)}
              className="bg-white/80 border-slate-200 text-slate-900 font-semibold"
            />
          </div>
        </div>
      </motion.div>

      {/* ITEMS TABLE & ENTRY */}
      <motion.div variants={itemVars} className="col-span-12 lg:col-span-8 glass-widget p-6 hover-lift flex flex-col justify-between">
        <div className="overflow-x-auto rounded-xl border border-slate-200/80 bg-white/50">
          <Table>
            <TableHeader className="bg-slate-100/80">
              <TableRow className="border-slate-200">
                <TableHead className="w-[280px] text-xs font-bold uppercase text-slate-700">Item (Press Space)</TableHead>
                <TableHead className="text-xs font-bold uppercase text-slate-700">Qty</TableHead>
                <TableHead className="text-xs font-bold uppercase text-slate-700">Rate ($)</TableHead>
                <TableHead className="text-xs font-bold uppercase text-slate-700">Disc ($)</TableHead>
                <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Total ($)</TableHead>
                <TableHead className="w-[50px]"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {items.map((item, idx) => {
                const selectedInv = inventoryList.find((i) => i.id === item.inventory_id);
                const belowCost =
                  selectedInv && invoiceType.includes("sale")
                    ? isSellingBelowCost(item.rate, selectedInv.cost_price)
                    : false;

                return (
                  <TableRow key={idx} className="border-slate-100 hover:bg-blue-50/30">
                    <TableCell>
                      <Popover open={openCommand === idx} onOpenChange={(open) => setOpenCommand(open ? idx : null)}>
                        <PopoverTrigger asChild>
                          <Button
                            variant="outline"
                            role="combobox"
                            className={cn(
                              "w-full justify-between font-medium text-slate-800 bg-white/80 border-slate-200",
                              !item.inventory_id && "text-slate-400"
                            )}
                            onKeyDown={(e) => {
                              if (e.key === " " || e.key === "Spacebar") {
                                e.preventDefault();
                                setOpenCommand(idx);
                              }
                            }}
                          >
                            {selectedInv ? selectedInv.name : "Select item..."}
                            <Search className="ml-2 h-4 w-4 shrink-0 opacity-50" />
                          </Button>
                        </PopoverTrigger>
                        <PopoverContent className="w-[300px] p-0" align="start">
                          <Command>
                            <CommandInput placeholder="Search inventory..." />
                            <CommandList>
                              <CommandEmpty>No item found.</CommandEmpty>
                              <CommandGroup>
                                {inventoryList.map((inv) => (
                                  <CommandItem
                                    key={inv.id}
                                    value={inv.name}
                                    onSelect={() => {
                                      updateItem(idx, "inventory_id", inv.id);
                                      setOpenCommand(null);
                                    }}
                                  >
                                    {inv.name} (Stock: {inv.stock_level})
                                  </CommandItem>
                                ))}
                              </CommandGroup>
                            </CommandList>
                          </Command>
                        </PopoverContent>
                      </Popover>
                    </TableCell>
                    <TableCell>
                      <Input
                        type="number"
                        min="1"
                        value={item.quantity || ""}
                        onChange={(e) => updateItem(idx, "quantity", parseFloat(e.target.value) || 0)}
                        className="bg-white/80 border-slate-200 text-slate-900 font-semibold text-center w-20"
                      />
                    </TableCell>
                    <TableCell className="relative">
                      <Input
                        type="number"
                        className={cn("bg-white/80 border-slate-200 text-slate-900 font-semibold w-24", belowCost && "text-rose-600 border-rose-400 focus-visible:ring-rose-500")}
                        value={item.rate || ""}
                        onChange={(e) => updateItem(idx, "rate", parseFloat(e.target.value) || 0)}
                      />
                      {belowCost && (
                        <span className="text-[10px] text-rose-600 font-bold block mt-0.5">Below Cost!</span>
                      )}
                    </TableCell>
                    <TableCell>
                      <Input
                        type="number"
                        className="bg-white/80 border-slate-200 text-slate-900 font-semibold w-20"
                        value={item.discount || ""}
                        onChange={(e) => updateItem(idx, "discount", parseFloat(e.target.value) || 0)}
                      />
                    </TableCell>
                    <TableCell className="text-right font-extrabold text-slate-900">
                      ${item.total.toFixed(2)}
                    </TableCell>
                    <TableCell>
                      <Button variant="ghost" size="icon" onClick={() => handleRemoveItem(idx)} className="hover:bg-rose-50 text-slate-400 hover:text-rose-600">
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })}
              {items.length === 0 && (
                <TableRow>
                  <TableCell colSpan={6} className="text-center py-8 text-slate-500 font-medium">
                    No items added yet. Click "+ Add Line Item" below to begin.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </div>
        <div className="pt-4">
          <Button onClick={handleAddItem} className="bg-blue-50 text-blue-600 hover:bg-blue-100 font-bold border border-blue-200 cursor-pointer">
            <Plus className="h-4 w-4 mr-2" /> Add Line Item
          </Button>
        </div>
      </motion.div>

      {/* SUMMARY SIDE PANEL */}
      <motion.div variants={itemVars} className="col-span-12 lg:col-span-4 glass-widget p-6 hover-lift flex flex-col justify-between border-blue-200/80">
        <h3 className="text-lg font-bold text-slate-900 border-b border-slate-200 pb-3 mb-4">Invoice Summary</h3>
        <div className="space-y-4">
          <div className="flex justify-between items-center text-sm">
            <span className="text-slate-600 font-medium">Subtotal</span>
            <span className="font-bold text-slate-900 text-base">${subtotal.toFixed(2)}</span>
          </div>
          <div className="flex items-center justify-between text-sm">
            <span className="text-slate-600 font-medium">Global Discount</span>
            <Input
              type="number"
              className="w-28 h-9 text-right bg-white/80 border-slate-200 text-slate-900 font-bold"
              value={globalDiscount || ""}
              onChange={(e) => setGlobalDiscount(parseFloat(e.target.value) || 0)}
            />
          </div>
          {isCashSale && (
            <div className="p-2.5 rounded-lg bg-amber-50 border border-amber-200 text-xs text-amber-800 font-semibold">
              Cash Sale Rounding Automatically Applied
            </div>
          )}
          {selectedAccount && selectedAccount.credit_limit > 0 && grandTotal > selectedAccount.credit_limit && (
            <div className="p-2.5 rounded-lg bg-rose-50 border border-rose-200 text-xs text-rose-700 font-bold">
              Warning: Invoice exceeds Account Credit Limit (${selectedAccount.credit_limit})!
            </div>
          )}
        </div>

        <div className="mt-6 pt-4 border-t border-slate-200/80">
          <div className="flex justify-between items-baseline mb-2">
            <span className="font-extrabold text-slate-900 text-lg">Grand Total</span>
            <span className="text-3xl font-extrabold text-blue-600">${grandTotal.toFixed(2)}</span>
          </div>
          <p className="text-xs text-slate-500 text-right">Net Payable Amount</p>
        </div>
      </motion.div>
    </motion.div>
  );
}

