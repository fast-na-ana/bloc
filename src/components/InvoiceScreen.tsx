import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
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
import { Trash2, Search } from "lucide-react";
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
    // Fetch accounts and inventory on mount
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
    
    // Auto-merge behavior is handled safely by the backend for matching date and account
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
      // Reset form
      setItems([]);
      setGlobalDiscount(0);
      setAccountId("");
    } catch (err) {
      alert("Error creating invoice: " + err);
    }
  };

  const subtotal = calculateSubtotal(items);
  // Apply rounding logic if it's a cash sale. 
  // We determine "cash sale" based on the account name or type (e.g. "Cash" account).
  const selectedAccount = accounts.find((a) => a.id === parseInt(accountId));
  const isCashSale =
    selectedAccount?.name.toLowerCase().includes("cash") || false;
    
  const grandTotal = calculateGrandTotal(subtotal, globalDiscount, isCashSale);

  return (
    <div className="flex flex-col h-full bg-background text-foreground p-8 space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-3xl font-semibold tracking-tight text-primary">
          New Invoice
        </h1>
        <Button onClick={handleSave} size="lg">
          Save & Auto-Merge
        </Button>
      </div>

      <div className="grid grid-cols-3 gap-6 bg-muted/30 p-6 rounded-xl border">
        <div className="space-y-2">
          <label className="text-sm font-medium text-muted-foreground">Type</label>
          <Select value={invoiceType} onValueChange={setInvoiceType}>
            <SelectTrigger>
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
          <label className="text-sm font-medium text-muted-foreground">Account</label>
          <Select value={accountId} onValueChange={setAccountId}>
            <SelectTrigger>
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
          <label className="text-sm font-medium text-muted-foreground">Date</label>
          <Input
            type="date"
            value={invoiceDate}
            onChange={(e) => setInvoiceDate(e.target.value)}
          />
        </div>
      </div>

      <div className="flex-1 overflow-auto rounded-xl border">
        <Table>
          <TableHeader className="bg-muted">
            <TableRow>
              <TableHead className="w-[300px]">Item (Press Space)</TableHead>
              <TableHead>Qty</TableHead>
              <TableHead>Rate</TableHead>
              <TableHead>Discount</TableHead>
              <TableHead className="text-right">Total</TableHead>
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
                <TableRow key={idx}>
                  <TableCell>
                    <Popover open={openCommand === idx} onOpenChange={(open) => setOpenCommand(open ? idx : null)}>
                      <PopoverTrigger asChild>
                        <Button
                          variant="outline"
                          role="combobox"
                          className={cn(
                            "w-full justify-between font-normal",
                            !item.inventory_id && "text-muted-foreground"
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
                    />
                  </TableCell>
                  <TableCell>
                    <Input
                      type="number"
                      className={cn(belowCost && "text-destructive border-destructive focus-visible:ring-destructive")}
                      value={item.rate || ""}
                      onChange={(e) => updateItem(idx, "rate", parseFloat(e.target.value) || 0)}
                    />
                    {belowCost && (
                      <span className="text-[10px] text-destructive absolute mt-1">Below Cost!</span>
                    )}
                  </TableCell>
                  <TableCell>
                    <Input
                      type="number"
                      value={item.discount || ""}
                      onChange={(e) => updateItem(idx, "discount", parseFloat(e.target.value) || 0)}
                    />
                  </TableCell>
                  <TableCell className="text-right font-medium">
                    {item.total.toFixed(2)}
                  </TableCell>
                  <TableCell>
                    <Button variant="ghost" size="icon" onClick={() => handleRemoveItem(idx)}>
                      <Trash2 className="h-4 w-4 text-muted-foreground" />
                    </Button>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
        <div className="p-4 border-t">
          <Button variant="secondary" onClick={handleAddItem}>
            + Add Row
          </Button>
        </div>
      </div>

      <div className="flex justify-end pt-4">
        <div className="w-80 space-y-3 bg-muted/30 p-6 rounded-xl border">
          <div className="flex justify-between text-sm text-muted-foreground">
            <span>Subtotal</span>
            <span>{subtotal.toFixed(2)}</span>
          </div>
          <div className="flex items-center justify-between text-sm">
            <span className="text-muted-foreground">Discount</span>
            <Input
              type="number"
              className="w-24 h-8 text-right"
              value={globalDiscount || ""}
              onChange={(e) => setGlobalDiscount(parseFloat(e.target.value) || 0)}
            />
          </div>
          {isCashSale && (
            <div className="flex justify-between text-xs text-amber-500 italic">
              <span>Cash Sale Rounding Applied</span>
            </div>
          )}
          {selectedAccount && selectedAccount.credit_limit > 0 && grandTotal > selectedAccount.credit_limit && (
            <div className="flex justify-between text-xs text-destructive italic">
              <span>Warning: Invoice exceeds Account Credit Limit!</span>
            </div>
          )}
          <div className="flex justify-between font-bold text-xl pt-2 border-t">
            <span>Total</span>
            <span>{grandTotal.toFixed(2)}</span>
          </div>
        </div>
      </div>
    </div>
  );
}
