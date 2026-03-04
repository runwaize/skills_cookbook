import { useNavigate } from 'react-router-dom';
import { ChefHat, UtensilsCrossed } from 'lucide-react';
import { useRole } from '@/contexts/RoleContext';
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card';

export function RoleSelectPage() {
  const { setRole } = useRole();
  const navigate = useNavigate();

  const select = async (role: 'chef' | 'cook') => {
    try {
      await setRole(role);
    } catch (err) {
      console.warn('Role selection error:', err);
    }
    navigate(role === 'chef' ? '/chef' : '/cook');
  };

  return (
    <div className="flex items-center justify-center min-h-screen bg-background p-8">
      <div className="max-w-2xl w-full space-y-8">
        <div className="text-center">
          <h1 className="text-3xl font-bold">Welcome to Skill Cookbook</h1>
          <p className="text-muted-foreground mt-2">Choose your role to get started</p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <Card
            className="cursor-pointer hover:border-primary transition-colors"
            onClick={() => select('chef')}
          >
            <CardHeader className="text-center">
              <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-primary/10 mb-4">
                <ChefHat className="h-8 w-8 text-primary" />
              </div>
              <CardTitle>Chef</CardTitle>
              <CardDescription>Creator mode</CardDescription>
            </CardHeader>
            <CardContent>
              <ul className="text-sm text-muted-foreground space-y-1">
                <li>Create and edit skills</li>
                <li>Push skills to server</li>
                <li>Discover skills from AI agents</li>
                <li>Manage libraries</li>
              </ul>
            </CardContent>
          </Card>

          <Card
            className="cursor-pointer hover:border-primary transition-colors"
            onClick={() => select('cook')}
          >
            <CardHeader className="text-center">
              <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-primary/10 mb-4">
                <UtensilsCrossed className="h-8 w-8 text-primary" />
              </div>
              <CardTitle>Cook</CardTitle>
              <CardDescription>Consumer mode</CardDescription>
            </CardHeader>
            <CardContent>
              <ul className="text-sm text-muted-foreground space-y-1">
                <li>Browse available skills</li>
                <li>Sync skills from server</li>
                <li>View skill manifest</li>
                <li>Monitor relay status</li>
              </ul>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
