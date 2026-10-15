import {createRoot} from 'react-dom/client';
import LoginPage from './LoginPage';
const root=document.getElementById('login-root');
if(root)createRoot(root).render(<LoginPage/>);
