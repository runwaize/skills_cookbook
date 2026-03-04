import { useState } from 'react';
import { ArrowDownToLine, RefreshCw } from 'lucide-react';
import { commands } from '@/lib/tauri';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';

export function CookSyncView() {
  const [syncing, setSyncing] = useState(false);
  const [result, setResult] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const sync = async () => {
    setSyncing(true);
    setError(null);
    setResult(null);
    try {
      const count = await commands.syncGuest();
      setResult(count);
    } catch (e) {
      setError(String(e));
    } finally {
      setSyncing(false);
    }
  };

  return (
    <div className="space-y-6 max-w-xl">
      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <ArrowDownToLine className="h-4 w-4" />
            Sync from Server
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-sm text-muted-foreground">
            Fetch the latest skill manifest from the server and update your local copy.
          </p>
          <Button onClick={sync} disabled={syncing}>
            {syncing ? (
              <>
                <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                Syncing...
              </>
            ) : (
              <>
                <ArrowDownToLine className="h-4 w-4 mr-2" />
                Sync Now
              </>
            )}
          </Button>
        </CardContent>
      </Card>

      {error && (
        <Card className="border-destructive">
          <CardContent className="p-4">
            <p className="text-sm text-destructive">{error}</p>
          </CardContent>
        </Card>
      )}

      {result !== null && (
        <Card className="border-primary/30">
          <CardContent className="p-4">
            <p className="text-sm">
              Manifest updated: <strong>{result}</strong> skill(s) synced.
            </p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
