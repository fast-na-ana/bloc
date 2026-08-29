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
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Printer, Eye } from "lucide-react";

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

interface InvoiceSummary {
  id: number;
  invoice_type: string;
  account_id: number;
  account_name: string;
  date: string;
  total: number;
  voided_at: string | null;
}



interface InvoiceDetailItem {
  id: number;
  inventory_name: string;
  quantity: number;
  rate: number;
  discount: number;
  total: number;
}

interface InvoiceDetail {
  id: number;
  invoice_type: string;
  account_id: number;
  account_name: string;
  date: string;
  subtotal: number;
  global_discount: number;
  grand_total: number;
  is_cash: boolean;
  voided_at: string | null;
  items: InvoiceDetailItem[];
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
  const [invoices, setInvoices] = useState<InvoiceSummary[]>([]);
  const [activeTab, setActiveTab] = useState("create");
  const [filterAccount, setFilterAccount] = useState("all");
  
  const [openCommand, setOpenCommand] = useState<number | null>(null);
  const [selectedInvoice, setSelectedInvoice] = useState<InvoiceDetail | null>(null);
  const [detailDialogOpen, setDetailDialogOpen] = useState(false);

  useEffect(() => {
    async function loadData() {
      try {
        const acc = await invoke<Account[]>("get_accounts");
        const inv = await invoke<Inventory[]>("get_inventory");
        setAccounts(acc);
        setInventoryList(inv);
        const inv_list = await invoke<InvoiceSummary[]>("get_invoices");
        setInvoices(inv_list);
      } catch (err) {
        console.error("Failed to load initial data", err);
      }
    }
    loadData();
  }, []);

  
  const handleViewInvoice = async (invoiceId: number) => {
    try {
      const detail = await invoke<InvoiceDetail>("get_invoice_details", { invoiceId });
      setSelectedInvoice(detail);
      setDetailDialogOpen(true);
    } catch (err) {
      console.error(err);
      alert("Failed to load invoice details: " + err);
    }
  };

  const handlePrint = () => {
    window.print();
  };

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
      const inv_list = await invoke<InvoiceSummary[]>("get_invoices");
      setInvoices(inv_list);
      setActiveTab("view");
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
    <motion.div variants={containerVars} initial="hidden" animate="show" className="flex flex-col gap-6 h-full">
      <Tabs defaultValue="create" value={activeTab} onValueChange={setActiveTab} className="w-full flex flex-col gap-6">
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
        <TabsList className="bg-slate-100/80 border border-slate-200/80 p-1 rounded-xl hidden md:flex">
          <TabsTrigger value="create" className="data-[state=active]:bg-blue-600 data-[state=active]:text-white font-semibold rounded-lg text-xs px-4">Create Invoice</TabsTrigger>
          <TabsTrigger value="view" className="data-[state=active]:bg-blue-600 data-[state=active]:text-white font-semibold rounded-lg text-xs px-4">View Invoices</TabsTrigger>
        </TabsList>
      </motion.div>

      <TabsContent value="create" className="mt-0 outline-none">
        <div className="grid grid-cols-12 gap-6 auto-rows-max">
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
        <div className="col-span-12 flex justify-end">
          <Button 
            onClick={handleSave} 
            size="lg"
            className="bg-blue-600 hover:bg-blue-700 text-white font-semibold shadow-md shadow-blue-600/20 transition-all cursor-pointer w-full md:w-auto mt-4"
          >
            <Check className="h-4 w-4 mr-2" /> Save & Auto-Merge
          </Button>
        </div>
        </div>
        </TabsContent>

        <TabsContent value="view" className="mt-0 outline-none flex-1">
          <motion.div variants={itemVars} className="glass-widget p-6 hover-lift flex flex-col h-full">
            <div className="flex flex-col sm:flex-row items-center justify-between gap-4 mb-4">
              <h2 className="text-lg font-bold text-slate-900 flex items-center gap-2">
                <Receipt className="h-5 w-5 text-blue-600" /> Issued Invoices
              </h2>
              <div className="flex items-center gap-2 w-full sm:w-auto">
                <Select value={filterAccount} onValueChange={setFilterAccount}>
                  <SelectTrigger className="bg-white/80 border-slate-200 font-semibold w-48">
                    <SelectValue placeholder="Filter by Party" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">All Parties</SelectItem>
                    {accounts.map(acc => (
                      <SelectItem key={acc.id} value={acc.id.toString()}>{acc.name}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="overflow-x-auto rounded-xl border border-slate-200/80 bg-white/50">
              <Table>
                <TableHeader className="bg-slate-100/80">
                  <TableRow className="border-slate-200">
                    <TableHead className="text-xs font-bold uppercase text-slate-700">Date</TableHead>
                    <TableHead className="text-xs font-bold uppercase text-slate-700">Invoice ID</TableHead>
                    <TableHead className="text-xs font-bold uppercase text-slate-700">Type</TableHead>
                    <TableHead className="text-xs font-bold uppercase text-slate-700">Party</TableHead>
                    <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Total ($)</TableHead>
                    <TableHead className="text-center text-xs font-bold uppercase text-slate-700">Status</TableHead>
                    <TableHead className="text-center text-xs font-bold uppercase text-slate-700 w-24">Action</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {invoices.filter(inv => filterAccount === 'all' || inv.account_id.toString() === filterAccount).map(inv => (
                    <TableRow key={inv.id} className="border-slate-100 hover:bg-blue-50/30">
                      <TableCell className="font-medium text-slate-800">{`${inv.date.split('T')[0]}`}</TableCell>
                      <TableCell className="font-bold text-blue-600">INV-{`${inv.id}`}</TableCell>
                      <TableCell className="capitalize text-slate-600 font-semibold">{`${inv.invoice_type.replace('_', ' ')}`}</TableCell>
                      <TableCell className="font-bold text-slate-900">{`${inv.account_name}`}</TableCell>
                      <TableCell className="text-right font-extrabold text-slate-900">${`${inv.total.toFixed(2)}`}</TableCell>
                      <TableCell className="text-center">
                        {inv.voided_at ? (
                          <span className="inline-block px-2.5 py-0.5 rounded-full text-xs font-extrabold bg-rose-50 text-rose-700 border border-rose-200">Voided</span>
                        ) : (
                          <span className="inline-block px-2.5 py-0.5 rounded-full text-xs font-extrabold bg-emerald-50 text-emerald-700 border border-emerald-200">Active</span>
                        )}
                      </TableCell>
                      <TableCell className="text-center">
                        <Button variant="ghost" size="sm" onClick={() => handleViewInvoice(inv.id)} className="text-blue-600 hover:bg-blue-50 font-bold">
                          <Eye className="h-4 w-4 mr-1" /> View
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                  {invoices.length === 0 && (
                    <TableRow>
                      <TableCell colSpan={6} className="text-center py-8 text-slate-500">No invoices found.</TableCell>
                    </TableRow>
                  )}
                </TableBody>
              </Table>
            </div>
          </motion.div>
        </TabsContent>
      </Tabs>

      {/* INVOICE DETAIL DIALOG */}
      <Dialog open={detailDialogOpen} onOpenChange={setDetailDialogOpen}>
        <DialogContent className="max-w-4xl glass-widget bg-white/95 border-slate-200 backdrop-blur-xl print:m-0 print:max-w-none print:shadow-none print:border-none print:bg-white print:h-screen print:w-screen print:rounded-none">
          <DialogHeader className="flex flex-row items-center justify-between border-b border-slate-200 pb-4 print:hidden">
            <DialogTitle className="text-2xl font-extrabold text-slate-900 flex items-center gap-2">
              <Receipt className="h-6 w-6 text-blue-600" />
              Invoice {selectedInvoice ? `INV-${selectedInvoice.id}` : ''}
            </DialogTitle>
            <Button onClick={handlePrint} className="bg-slate-900 hover:bg-slate-800 text-white font-bold cursor-pointer print:hidden">
              <Printer className="h-4 w-4 mr-2" /> Print
            </Button>
          </DialogHeader>
          
          {selectedInvoice && (
            <div className="mt-4 print:p-8 print:m-0 print:bg-white print:text-black">
              {/* Print Header */}
              <div className="hidden print:flex justify-between items-center mb-10 border-b border-slate-200 pb-6">
                 <div>
                   <h1 className="text-3xl font-black text-slate-900">INVOICE</h1>
                   <p className="text-slate-500 font-bold mt-1">INV-{selectedInvoice.id}</p>
                 </div>
                 <div className="text-right">
                   <h2 className="text-xl font-bold text-slate-800">Your Company Name</h2>
                   <p className="text-slate-500">123 Business Rd, City, Country</p>
                 </div>
              </div>

              {/* Header Info */}
              <div className="grid grid-cols-2 gap-8 mb-8">
                <div>
                  <p className="text-sm font-bold text-slate-500 uppercase tracking-wider mb-1">Billed To</p>
                  <p className="text-lg font-extrabold text-slate-900">{selectedInvoice.account_name}</p>
                </div>
                <div className="text-right">
                  <p className="text-sm font-bold text-slate-500 uppercase tracking-wider mb-1">Invoice Details</p>
                  <p className="text-slate-900"><span className="font-semibold text-slate-600">Date:</span> {selectedInvoice.date.split('T')[0]}</p>
                  <p className="text-slate-900"><span className="font-semibold text-slate-600">Type:</span> <span className="capitalize">{selectedInvoice.invoice_type.replace('_', ' ')}</span></p>
                  <p className="text-slate-900"><span className="font-semibold text-slate-600">Status:</span> {selectedInvoice.voided_at ? <span className="text-rose-600 font-bold">Voided</span> : <span className="text-emerald-600 font-bold">Active</span>}</p>
                </div>
              </div>

              {/* Items Table */}
              <div className="overflow-x-auto rounded-xl border border-slate-200 bg-white mb-6 print:border-none print:shadow-none">
                <Table>
                  <TableHeader className="bg-slate-100 print:bg-slate-50">
                    <TableRow>
                      <TableHead className="text-xs font-bold uppercase text-slate-700">Item</TableHead>
                      <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Qty</TableHead>
                      <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Rate</TableHead>
                      <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Discount</TableHead>
                      <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Total</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {selectedInvoice.items.map(item => (
                      <TableRow key={item.id} className="print:border-b-slate-100">
                        <TableCell className="font-bold text-slate-900">{item.inventory_name}</TableCell>
                        <TableCell className="text-right font-medium text-slate-700">{item.quantity}</TableCell>
                        <TableCell className="text-right font-medium text-slate-700">${item.rate.toFixed(2)}</TableCell>
                        <TableCell className="text-right font-medium text-slate-700">{item.discount > 0 ? `$${item.discount.toFixed(2)}` : '-'}</TableCell>
                        <TableCell className="text-right font-extrabold text-slate-900">${item.total.toFixed(2)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>

              {/* Totals */}
              <div className="flex justify-end mt-8">
                <div className="w-64 space-y-3">
                  <div className="flex justify-between text-slate-600 font-medium">
                    <span>Subtotal</span>
                    <span>${selectedInvoice.subtotal.toFixed(2)}</span>
                  </div>
                  {selectedInvoice.global_discount > 0 && (
                    <div className="flex justify-between text-slate-600 font-medium">
                      <span>Global Discount</span>
                      <span className="text-rose-600">-${selectedInvoice.global_discount.toFixed(2)}</span>
                    </div>
                  )}
                  <div className="flex justify-between items-baseline pt-3 border-t border-slate-200 print:border-slate-300">
                    <span className="font-extrabold text-slate-900">Grand Total</span>
                    <span className="text-2xl font-extrabold text-blue-600 print:text-slate-900">${selectedInvoice.grand_total.toFixed(2)}</span>
                  </div>
                </div>
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>

    </motion.div>
  );
}
