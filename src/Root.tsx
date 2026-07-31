import App from './App';
import { CompanionWindow } from './features/floating-companion/CompanionWindow';

type RootProps = {
  windowLabel: string;
};

export default function Root({ windowLabel }: RootProps) {
  return windowLabel === 'companion' ? <CompanionWindow /> : <App />;
}
