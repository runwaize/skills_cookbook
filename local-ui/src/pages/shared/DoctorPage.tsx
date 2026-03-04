import { useEffect, useState } from 'react';
import { Stethoscope, CheckCircle, AlertTriangle, XCircle, RefreshCw } from 'lucide-react';
import { commands, type DiagnosticCheck } from '@/lib/tauri';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';

const statusIcon = {
  ok: <CheckCircle className="h-4 w-4 text-success" />,
  warning: <AlertTriangle className="h-4 w-4 text-warning" />,
  error: <XCircle className="h-4 w-4 text-destructive" />,
};

export function DoctorPage() {
  const [checks, setChecks] = useState<DiagnosticCheck[]>([]);
  const [loading, setLoading] = useState(true);

  const run = () => {
    setLoading(true);
    commands
      .runDoctor()
      .then(setChecks)
      .catch(() => setChecks([]))
      .finally(() => setLoading(false));
  };

  useEffect(run, []);

  return (
    <div className="space-y-4 max-w-xl">
      <div className="flex justify-end">
        <Button variant="ghost" size="sm" onClick={run} disabled={loading}>
          <RefreshCw className={`h-4 w-4 mr-1 ${loading ? 'animate-spin' : ''}`} />
          Re-run
        </Button>
      </div>

      {loading ? (
        <div className="space-y-3">
          {[1, 2, 3, 4].map((i) => (
            <Skeleton key={i} className="h-14 w-full" />
          ))}
        </div>
      ) : (
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Stethoscope className="h-4 w-4" />
              Diagnostic Results
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              {checks.map((check, i) => (
                <div
                  key={i}
                  className="flex items-start gap-3 py-2 border-b last:border-0"
                >
                  <div className="mt-0.5">{statusIcon[check.status]}</div>
                  <div>
                    <p className="text-sm font-medium">{check.name}</p>
                    <p className="text-xs text-muted-foreground">{check.message}</p>
                  </div>
                </div>
              ))}
              {checks.length === 0 && (
                <p className="text-sm text-muted-foreground">No checks returned.</p>
              )}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
