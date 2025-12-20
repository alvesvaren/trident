import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import init from 'trident-core'
import wasmUrl from 'trident-core/trident_core_bg.wasm?url'

// Initialize WASM before rendering
init(wasmUrl).then(() => {
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )
})
