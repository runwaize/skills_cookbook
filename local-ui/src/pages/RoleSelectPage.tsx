import { Navigate } from 'react-router-dom';

/** Role selector is no longer needed — both Chef and Cook are always available. */
export function RoleSelectPage() {
  return <Navigate to="/chef" replace />;
}
