import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { EmptyState } from "./ui/EmptyState";
import { PackageOpen } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";

interface Inventory {
  id: number;
  name: string;
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

export function InventoryScreen() {
  const [inventoryList, setInventoryList] = useState<Inventory[]>([]);
  const [selectedItem, setSelectedItem] = useState<Inventory | null>(null);
  const [ledger, setLedger] = useState<ItemLedgerEntry[]>([]);
  const [dialogOpen, setDialogOpen] = useState(false);

  // Form State
  const [name, setName] = useState("");
  const [costPrice, setCostPrice] = useState<number | "">("");
  const [salePrice, setSalePrice] = useState<number | "">("");

  useEffect(() => {
    invoke<Inventory[]>("get_inventory").then(setInventoryList).catch(console.error);
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
        stock_level: 0,
        cost_price: Number(costPrice),
        sale_price: Number(salePrice)
      });
      setName("");
      setCostPrice("");
      setSalePrice("");
      const data = await invoke<Inventory[]>("get_inventory");
      setInventoryList(data);
    } catch (err) {
      console.error(err);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-2">
        <h2 className="text-3xl font-bold tracking-tight text-primary">Inventory Management</h2>
        <p className="text-muted-foreground">Monitor real-time stock levels and view transaction histories.</p>
      </div>

      <div className="grid md:grid-cols-3 gap-6">
        <Card className="md:col-span-1 h-fit hover-lift">
          <CardHeader>
            <CardTitle>Add New Item</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Item Name</label>
              <Input value={name} onChange={e => setName(e.target.value)} placeholder="e.g. Widget A" />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <label className="text-sm font-medium">Cost Price</label>
                <Input type="number" value={costPrice} onChange={e => setCostPrice(parseFloat(e.target.value) || "")} />
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Sale Price</label>
                <Input type="number" value={salePrice} onChange={e => setSalePrice(parseFloat(e.target.value) || "")} />
              </div>
            </div>
            <Button className="w-full mt-2" onClick={handleCreate}>Create Item</Button>
          </CardContent>
        </Card>

        <Card className="md:col-span-2 hover-lift">
          <CardHeader>
            <CardTitle>Current Stock</CardTitle>
          </CardHeader>
          <CardContent>
            {inventoryList.length === 0 ? (
              <EmptyState icon={PackageOpen} title="No Inventory" description="Your inventory is currently empty. Add products to get started." />
            ) : (
              <div className="rounded-md border">
                <Table>
                  <TableHeader className="bg-muted/50">
                    <TableRow>
                      <TableHead>Item Name</TableHead>
                      <TableHead className="text-right">Stock Level</TableHead>
                      <TableHead className="text-right">Cost Price</TableHead>
                      <TableHead className="text-right">Sale Price</TableHead>
                      <TableHead className="w-[80px]"></TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {inventoryList.map(item => (
                      <TableRow key={item.id}>
                        <TableCell className="font-medium">{item.name}</TableCell>
                        <TableCell className={`text-right font-bold ${item.stock_level < 10 ? 'text-amber-600' : ''}`}>
                          {item.stock_level}
                        </TableCell>
                        <TableCell className="text-right text-muted-foreground">{item.cost_price.toFixed(2)}</TableCell>
                        <TableCell className="text-right">{item.sale_price.toFixed(2)}</TableCell>
                        <TableCell>
                          <Button variant="ghost" size="sm" onClick={() => handleViewLedger(item)}>History</Button>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-3xl max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Stock Ledger: {selectedItem?.name}</DialogTitle>
          </DialogHeader>
          <div className="mt-4 rounded-md border">
            <Table>
              <TableHeader className="bg-muted/50">
                <TableRow>
                  <TableHead>Date</TableHead>
                  <TableHead>Description</TableHead>
                  <TableHead className="text-right">IN (+)</TableHead>
                  <TableHead className="text-right">OUT (-)</TableHead>
                  <TableHead className="text-right">Balance</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {ledger.map((entry, idx) => (
                  <TableRow key={idx}>
                    <TableCell>{entry.date.split('T')[0]}</TableCell>
                    <TableCell className="capitalize">{entry.description.replace('_', ' ')}</TableCell>
                    <TableCell className="text-right text-emerald-600">{entry.quantity_in > 0 ? entry.quantity_in : "-"}</TableCell>
                    <TableCell className="text-right text-destructive">{entry.quantity_out > 0 ? entry.quantity_out : "-"}</TableCell>
                    <TableCell className="text-right font-semibold">{entry.balance}</TableCell>
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
    </div>
  );
}
