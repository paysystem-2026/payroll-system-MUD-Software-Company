import { ButtonHTMLAttributes, ReactNode } from "react";

type Variant = "primary" | "secondary" | "ghost";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  children: ReactNode;
}

const variants: Record<Variant, string> = {
  primary:
    "bg-[#4a8b3f] text-white shadow-[0_8px_22px_rgba(74,139,63,0.16)] hover:bg-[#568f4b] hover:-translate-y-[1px]",
  secondary:
    "border border-[#303030] bg-[#202020] text-[#efefef] hover:border-[#3c3c3c] hover:bg-[#272727] hover:-translate-y-[1px]",
  ghost:
    "bg-transparent text-[#8b8b8b] hover:bg-[#1e1e1e] hover:text-white",
};

export function Button({ variant = "primary", children, className = "", ...props }: ButtonProps) {
  return (
    <button
      className={`inline-flex items-center justify-center gap-2 rounded-xl px-4 py-2.5 text-[12px] font-semibold transition-all duration-200 disabled:cursor-not-allowed disabled:opacity-50 ${variants[variant]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
}
