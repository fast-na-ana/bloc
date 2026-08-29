import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion } from "framer-motion";
import { EmptyState } from "./ui/EmptyState";
import { PackageOpen, Package, Plus, History, Minus, Check } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

interface Inventory {
  id: number;
  name: string;
  item_type: string;
  stock_level: number;
  cost_price: number;
  sale_price: number;
}

interface ItemLedgerEntry {
  date: string;
  description: string;
  quantity_in: number;
  quantity_out: number;
  balance: number;
}

const containerVars = {
  hidden: { opacity: 0 },
  show: { opacity: 1, transition: { staggerChildren: 0.05 } }
} as any;

const itemVars = {
  hidden: { opacity: 0, y: 15 },
  show: { opacity: 1, y: 0, transition: { duration: 0.4, ease: "easeOut" } }
} as any;

export function InventoryScreen() {
  const [inventoryList, setInventoryList] = useState<Inventory[]>([]);
  const [selectedItem, setSelectedItem] = useState<Inventory | null>(null);
  const [ledger, setLedger] = useState<ItemLedgerEntry[]>([]);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [adjustDialogOpen, setAdjustDialogOpen] = useState(false);
  
  const [activeTab, setActiveTab] = useState("raw_material");

  // Create Item Form
  const [name, setName] = useState("");
  const [costPrice, setCostPrice] = useState<number | "">("");
  const [salePrice, setSalePrice] = useState<number | "">("");

  // Adjust Form
  const [adjustItemId, setAdjustItemId] = useState<string>("");
  const [adjustQty, setAdjustQty] = useState<number | "">("");

  const loadData = async () => {
    try {
      const data = await invoke<Inventory[]>("get_inventory");
      setInventoryList(data);
    } catch (err) {
      console.error(err);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleViewLedger = async (item: Inventory) => {
    setSelectedItem(item);
    try {
      const data = await invoke<ItemLedgerEntry[]>("get_item_ledger", { inventoryId: item.id });
      setLedger(data);
      setDialogOpen(true);
    } catch (err) {
      console.error(err);
    }
  };

  const handleCreate = async () => {
    if (!name || costPrice === "" || salePrice === "") return;
    try {
      await invoke("create_inventory_item", {
        name,
        itemType: activeTab,
        stockLevel: 0,
        costPrice: Number(costPrice),
        salePrice: Number(salePrice)
      });
      setName("");
      setCostPrice("");
      setSalePrice("");
      loadData();
    } catch (err) {
      console.error(err);
    }
  };

  const handleAdjustStock = async () => {
    if (!adjustItemId || adjustQty === "" || Number(adjustQty) <= 0) return;
    
    // For raw material (consumption), negative quantity. For product (production), positive quantity.
    const qtyChange = activeTab === "raw_material" ? -Number(adjustQty) : Number(adjustQty);
    const reason = activeTab === "raw_material" ? "Log Consumption" : "Log Production";
    
    try {
      await invoke("adjust_inventory_stock", {
        inventoryId: Number(adjustItemId),
        qtyChange,
        reason,
        userId: 1 // TODO: get real user id
      });
      setAdjustDialogOpen(false);
      setAdjustQty("");
      setAdjustItemId("");
      loadData();
    } catch (err) {
      console.error(err);
      alert(err);
    }
  };

  const filteredList = inventoryList.filter(i => i.item_type === activeTab);

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
            <Package className="text-blue-600 h-5 w-5" />
          </div>
          <div>
            <h1 className="text-2xl font-extrabold text-slate-900 leading-tight">
              Factory Inventory
            </h1>
            <p className="text-slate-500 text-sm font-medium">Manage Raw Materials & Finished Products</p>
          </div>
        </div>
      </motion.div>

      {/* FORM CARD */}
      <motion.div variants={itemVars} className="col-span-12 lg:col-span-4 glass-widget p-6 hover-lift flex flex-col justify-between">
        <div>
          <h2 className="text-lg font-bold text-slate-900 border-b border-slate-200 pb-3 mb-4">
            Add New {activeTab === 'raw_material' ? 'Raw Material' : 'Finished Product'}
          </h2>
          <div className="space-y-4">
            <div className="space-y-1.5">
              <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Item Name</label>
              <Input value={name} onChange={e => setName(e.target.value)} placeholder="e.g. Steel Pipe 10mm" className="bg-white/80 border-slate-200 text-slate-900 font-semibold" />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Cost Price ($)</label>
                <Input type="number" value={costPrice} onChange={e => setCostPrice(parseFloat(e.target.value) || "")} className="bg-white/80 border-slate-200 text-slate-900 font-semibold" />
              </div>
              <div className="space-y-1.5">
                <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Sale Price ($)</label>
                <Input type="number" value={salePrice} onChange={e => setSalePrice(parseFloat(e.target.value) || "")} className="bg-white/80 border-slate-200 text-slate-900 font-semibold" />
              </div>
            </div>
          </div>
        </div>

        <div className="pt-6">
          <Button onClick={handleCreate} className="w-full bg-blue-600 hover:bg-blue-700 text-white font-semibold shadow-md shadow-blue-600/20 transition-all cursor-pointer">
            <Plus className="h-4 w-4 mr-2" /> Register Item
          </Button>
        </div>
      </motion.div>

      {/* STOCK TABLE CARD */}
      <motion.div variants={itemVars} className="col-span-12 lg:col-span-8 glass-widget p-6 hover-lift flex flex-col">
        <Tabs defaultValue="raw_material" value={activeTab} onValueChange={setActiveTab} className="w-full">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between mb-6 gap-4">
            <TabsList className="bg-slate-100/80 border border-slate-200/80 p-1 rounded-xl">
              <TabsTrigger value="raw_material" className="data-[state=active]:bg-blue-600 data-[state=active]:text-white font-semibold rounded-lg text-xs px-4">Raw Materials</TabsTrigger>
              <TabsTrigger value="product" className="data-[state=active]:bg-blue-600 data-[state=active]:text-white font-semibold rounded-lg text-xs px-4">Finished Products</TabsTrigger>
            </TabsList>
            
            <Button 
              onClick={() => { setAdjustItemId(""); setAdjustQty(""); setAdjustDialogOpen(true); }}
              className={`font-semibold shadow-md transition-all cursor-pointer ${activeTab === 'raw_material' ? 'bg-rose-600 hover:bg-rose-700 shadow-rose-600/20 text-white' : 'bg-emerald-600 hover:bg-emerald-700 shadow-emerald-600/20 text-white'}`}
            >
              {activeTab === 'raw_material' ? <><Minus className="h-4 w-4 mr-2"/> Log Consumption</> : <><Plus className="h-4 w-4 mr-2"/> Log Production</>}
            </Button>
          </div>

          {filteredList.length === 0 ? (
            <EmptyState icon={PackageOpen} title="No Items" description="There are no items registered in this category." />
          ) : (
            <div className="overflow-x-auto rounded-xl border border-slate-200/80 bg-white/50">
              <Table>
                <TableHeader className="bg-slate-100/80">
                  <TableRow className="border-slate-200">
                    <TableHead className="text-xs font-bold uppercase text-slate-700">Item Name</TableHead>
                    <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Stock Level</TableHead>
                    <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Cost Price</TableHead>
                    <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Sale Price</TableHead>
                    <TableHead className="w-[100px] text-xs font-bold uppercase text-slate-700 text-center">Action</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {filteredList.map(item => (
                    <TableRow key={item.id} className="border-slate-100 hover:bg-blue-50/30">
                      <TableCell className="font-bold text-slate-900">{item.name}</TableCell>
                      <TableCell className="text-right">
                        <span className={`inline-block px-2.5 py-0.5 rounded-full text-xs font-extrabold ${item.stock_level < 10 ? 'bg-rose-50 text-rose-700 border border-rose-200' : 'bg-blue-50 text-blue-700 border border-blue-200'}`}>
                          {item.stock_level} units
                        </span>
                      </TableCell>
                      <TableCell className="text-right text-slate-600 font-medium">${item.cost_price.toFixed(2)}</TableCell>
                      <TableCell className="text-right font-extrabold text-slate-900">${item.sale_price.toFixed(2)}</TableCell>
                      <TableCell className="text-center">
                        <Button variant="ghost" size="sm" onClick={() => handleViewLedger(item)} className="text-blue-600 hover:bg-blue-50 font-bold">
                          <History className="h-4 w-4 mr-1" /> History
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </Tabs>
      </motion.div>

      {/* DIALOG LEDGER */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-3xl glass-widget bg-white/95 border-slate-200 backdrop-blur-xl">
          <DialogHeader>
            <DialogTitle className="text-xl font-extrabold text-slate-900 flex items-center gap-2">
              <Package className="h-5 w-5 text-blue-600" />
              Stock Ledger: {selectedItem?.name}
            </DialogTitle>
          </DialogHeader>
          <div className="mt-2 overflow-x-auto rounded-xl border border-slate-200 bg-white/80">
            <Table>
              <TableHeader className="bg-slate-100">
                <TableRow>
                  <TableHead className="text-xs font-bold uppercase text-slate-700">Date</TableHead>
                  <TableHead className="text-xs font-bold uppercase text-slate-700">Description</TableHead>
                  <TableHead className="text-right text-xs font-bold uppercase text-slate-700">IN (+)</TableHead>
                  <TableHead className="text-right text-xs font-bold uppercase text-slate-700">OUT (-)</TableHead>
                  <TableHead className="text-right text-xs font-bold uppercase text-slate-700">Balance</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {ledger.map((entry, idx) => (
                  <TableRow key={idx} className="border-slate-100 hover:bg-blue-50/20">
                    <TableCell className="font-medium text-slate-800">{entry.date.split('T')[0]}</TableCell>
                    <TableCell className="capitalize font-semibold text-slate-700">{entry.description.replace('_', ' ')}</TableCell>
                    <TableCell className="text-right font-extrabold text-emerald-600">{entry.quantity_in > 0 ? `+${entry.quantity_in}` : "-"}</TableCell>
                    <TableCell className="text-right font-extrabold text-rose-600">{entry.quantity_out > 0 ? `-${entry.quantity_out}` : "-"}</TableCell>
                    <TableCell className="text-right font-extrabold text-slate-900">{entry.balance}</TableCell>
                  </TableRow>
                ))}
                {ledger.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={5} className="p-0 border-0">
                      <EmptyState icon={PackageOpen} title="No Ledger Entries" description="There is no history for this item." />
                    </TableCell>
                  </TableRow>
                ) : null}
              </TableBody>
            </Table>
          </div>
        </DialogContent>
      </Dialog>
      
      {/* ADJUST STOCK DIALOG */}
      <Dialog open={adjustDialogOpen} onOpenChange={setAdjustDialogOpen}>
        <DialogContent className="max-w-md glass-widget bg-white/95 border-slate-200 backdrop-blur-xl">
          <DialogHeader>
            <DialogTitle className="text-xl font-extrabold text-slate-900 flex items-center gap-2">
              {activeTab === 'raw_material' ? 'Log Raw Material Consumption' : 'Log Finished Product'}
            </DialogTitle>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-1.5">
              <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Select Item</label>
              <Select value={adjustItemId} onValueChange={setAdjustItemId}>
                <SelectTrigger className="bg-white border-slate-200 text-slate-900 font-semibold">
                  <SelectValue placeholder="Select..." />
                </SelectTrigger>
                <SelectContent>
                  {filteredList.map(item => (
                    <SelectItem key={item.id} value={item.id.toString()}>{item.name} (Stock: {item.stock_level})</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <label className="text-xs font-bold uppercase tracking-wider text-slate-600">Quantity {activeTab === 'raw_material' ? 'Consumed' : 'Produced'}</label>
              <Input type="number" value={adjustQty} onChange={e => setAdjustQty(parseFloat(e.target.value) || "")} placeholder="Enter quantity" className="bg-white border-slate-200 text-slate-900 font-semibold" />
            </div>
            <Button onClick={handleAdjustStock} className={`w-full text-white font-semibold ${activeTab === 'raw_material' ? 'bg-rose-600 hover:bg-rose-700' : 'bg-emerald-600 hover:bg-emerald-700'}`}>
              <Check className="h-4 w-4 mr-2" /> Confirm
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </motion.div>
  );
}
