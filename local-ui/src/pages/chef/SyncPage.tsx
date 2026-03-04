import { useState } from 'react';
import { RefreshCw, GitCommit, Upload } from 'lucide-react';
import { commands, type SyncResult } from '@/lib/tauri';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';

export function SyncPage() {
  const [noPush, setNoPush] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [result, setResult] = useState<SyncResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const sync = async () => {
    setSyncing(true);
    setError(null);
    setResult(null);
    try {
      const r = await commands.syncChef(noPush);
      setResult(r);
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
            <RefreshCw className="h-4 w-4" />
            Sync Chef
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-sm text-muted-foreground">
            Commit local changes and optionally push skills to the server.
          </p>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              className="checkbox checkbox-sm"
              checked={noPush}
              onChange={(e) => setNoPush(e.target.checked)}
            />
            <span className="text-sm">Skip push (commit only)</span>
          </label>
          <Button onClick={sync} disabled={syncing}>
            {syncing ? (
              <>
                <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                Syncing...
              </>
            ) : (
              <>
                <Upload className="h-4 w-4 mr-2" />
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

      {result && (
        <Card className="border-primary/30">
          <CardHeader>
            <CardTitle className="text-base">Sync Complete</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div className="flex items-center gap-2">
                <GitCommit className="h-4 w-4" />
                <span>Committed: {result.committed ? 'Yes' : 'No changes'}</span>
              </div>
              <div className="flex items-center gap-2">
                <Upload className="h-4 w-4" />
                <span>Pushed: {result.pushed ? 'Yes' : 'No'}</span>
              </div>
              <div>
                <span className="text-muted-foreground">Skills pushed:</span> {result.skills_pushed}
              </div>
              <div>
                <span className="text-muted-foreground">Guest updated:</span>{' '}
                {result.guest_skills_updated}
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
