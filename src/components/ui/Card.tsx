import { ReactNode } from "react";

interface CardProps {
  children: ReactNode;
  className?: string;
}

export function Card({ children, className = "" }: CardProps) {
  return (
    <div
      className={`rounded-2xl border border-[#292929] bg-[#151515] shadow-[0_10px_30px_rgba(0,0,0,0.16)] transition-all duration-200 ${className}`}
    >
      {children}
    </div>
  );
}
