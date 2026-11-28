import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import { PenApp } from './pen/PenApp.tsx'
import { lockPageScroll } from './lib/pageScroll.ts'
import { WINDOW_ROLE } from './platform/host.ts'

lockPageScroll(document)

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    {/* The pen window is the same bundle opened as `?window=pen` (ADR 0096 A10). */}
    {WINDOW_ROLE === 'pen' ? <PenApp /> : <App />}
  </StrictMode>,
)
