/**
 * レイアウト（コンポーネント配置）のカスタマイズシーム。
 *
 * MVP では固定レイアウトのみ。将来ユーザーがパネル配置を変更できるよう、
 * レイアウト設定をドメイン状態から独立した Context として先に用意する。
 */

import { createContext, useContext, useMemo, useState, type ReactNode } from "react";

/** パネルの並び（将来はドラッグで並べ替え可能にする）。 */
export interface WorkspaceLayout {
  /** ファイル一覧パネルを左右どちらに置くか。 */
  listSide: "left" | "right";
}

const defaultLayout: WorkspaceLayout = { listSide: "left" };

interface LayoutContextValue {
  layout: WorkspaceLayout;
  setLayout: (l: WorkspaceLayout) => void;
}

const LayoutContext = createContext<LayoutContextValue | null>(null);

export function LayoutProvider({ children }: { children: ReactNode }) {
  const [layout, setLayout] = useState<WorkspaceLayout>(defaultLayout);
  const value = useMemo(() => ({ layout, setLayout }), [layout]);
  return (
    <LayoutContext.Provider value={value}>{children}</LayoutContext.Provider>
  );
}

export function useLayout(): LayoutContextValue {
  const ctx = useContext(LayoutContext);
  if (!ctx) throw new Error("useLayout must be used within a LayoutProvider");
  return ctx;
}
