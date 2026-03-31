import { type ReactNode } from "react";

interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description: string;
  action?: ReactNode;
}

export function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-20 px-6">
      <div className="mb-8 text-neutral-700">{icon}</div>
      <h3 className="text-sm font-light text-neutral-300 mb-2">{title}</h3>
      <p className="text-xs text-neutral-600 font-light text-center max-w-xs leading-relaxed">
        {description}
      </p>
      {action && <div className="mt-6">{action}</div>}
    </div>
  );
}

/* ── SVG Illustrations ─────────────────────────────────────────────────── */
/* Minimalist, geometric, monochrome + gold accent (#d4af37).              */
/* Each illustration is 80x80, uses thin strokes and subtle fills.         */

export function SocialIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Speech bubbles */}
      <rect x="8" y="16" width="40" height="28" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.4" />
      <line x1="16" y1="26" x2="38" y2="26" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <line x1="16" y1="32" x2="32" y2="32" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <rect x="32" y="36" width="40" height="28" rx="2" stroke="#d4af37" strokeWidth="1" opacity="0.5" />
      <line x1="40" y1="46" x2="62" y2="46" stroke="#d4af37" strokeWidth="1" opacity="0.3" />
      <line x1="40" y1="52" x2="56" y2="52" stroke="#d4af37" strokeWidth="1" opacity="0.3" />
      {/* Connection dots */}
      <circle cx="28" cy="30" r="1" fill="#d4af37" opacity="0.6" />
      <circle cx="52" cy="50" r="1" fill="#d4af37" opacity="0.6" />
    </svg>
  );
}

export function MessagesIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Lock + envelope */}
      <rect x="16" y="24" width="48" height="36" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <path d="M16 28L40 44L64 28" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      {/* Lock icon in center */}
      <rect x="34" y="38" width="12" height="10" rx="1" stroke="#d4af37" strokeWidth="1" opacity="0.5" />
      <path d="M37 38V34C37 31.8 38.3 30 40 30C41.7 30 43 31.8 43 34V38" stroke="#d4af37" strokeWidth="1" opacity="0.5" fill="none" />
      <circle cx="40" cy="44" r="1" fill="#d4af37" opacity="0.6" />
    </svg>
  );
}

export function FilesIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Stacked file icons */}
      <rect x="22" y="20" width="36" height="44" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <rect x="18" y="16" width="36" height="44" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <path d="M18 16H46L54 24V60H18V16Z" stroke="currentColor" strokeWidth="1" opacity="0.3" fill="none" />
      {/* Fold corner */}
      <path d="M46 16V24H54" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      {/* Content lines */}
      <line x1="26" y1="32" x2="46" y2="32" stroke="currentColor" strokeWidth="1" opacity="0.15" />
      <line x1="26" y1="38" x2="42" y2="38" stroke="currentColor" strokeWidth="1" opacity="0.15" />
      <line x1="26" y1="44" x2="38" y2="44" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      {/* Upload arrow */}
      <path d="M36 56V50M36 50L32 54M36 50L40 54" stroke="#d4af37" strokeWidth="1" opacity="0.5" />
    </svg>
  );
}

export function GovernanceIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Ballot box */}
      <rect x="20" y="28" width="40" height="32" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <rect x="34" y="28" width="12" height="3" rx="1" stroke="#d4af37" strokeWidth="1" opacity="0.5" />
      {/* Ballot paper going in */}
      <rect x="32" y="16" width="16" height="16" rx="1" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      <line x1="36" y1="22" x2="44" y2="22" stroke="#d4af37" strokeWidth="1" opacity="0.3" />
      <line x1="36" y1="26" x2="42" y2="26" stroke="#d4af37" strokeWidth="1" opacity="0.3" />
      {/* Checkmark on ballot */}
      <path d="M36 24L38 26L44 20" stroke="#d4af37" strokeWidth="1" opacity="0.5" />
      {/* Vote bars */}
      <rect x="26" y="40" width="20" height="3" rx="1" fill="#d4af37" opacity="0.2" />
      <rect x="26" y="46" width="14" height="3" rx="1" fill="currentColor" opacity="0.1" />
      <rect x="26" y="52" width="24" height="3" rx="1" fill="currentColor" opacity="0.1" />
    </svg>
  );
}

export function MarketplaceIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Storefront */}
      <rect x="16" y="32" width="48" height="32" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      {/* Awning */}
      <path d="M14 32H66" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <path d="M14 32C14 28 20 24 26 24" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <path d="M26 24C32 24 32 32 40 32" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <path d="M40 32C48 32 48 24 54 24" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <path d="M54 24C60 24 66 28 66 32" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      {/* Tag / price */}
      <circle cx="40" cy="48" r="8" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      <text x="40" y="51" textAnchor="middle" fontSize="8" fontFamily="monospace" fill="#d4af37" opacity="0.5">$</text>
      {/* Door */}
      <rect x="34" y="52" width="12" height="12" rx="1" stroke="currentColor" strokeWidth="1" opacity="0.2" />
    </svg>
  );
}

export function WalletIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Wallet body */}
      <rect x="12" y="24" width="48" height="36" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      {/* Flap */}
      <path d="M12 32H60" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      {/* Card slot */}
      <rect x="48" y="38" width="16" height="12" rx="6" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      <circle cx="56" cy="44" r="3" stroke="#d4af37" strokeWidth="1" opacity="0.5" />
      {/* Coins */}
      <circle cx="28" cy="44" r="6" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <circle cx="38" cy="44" r="6" stroke="currentColor" strokeWidth="1" opacity="0.15" />
      {/* Gold coin accent */}
      <circle cx="33" cy="42" r="4" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
    </svg>
  );
}

export function AIIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Brain/neural pattern */}
      <circle cx="40" cy="40" r="20" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <circle cx="40" cy="40" r="12" stroke="currentColor" strokeWidth="1" opacity="0.15" />
      {/* Neural nodes */}
      <circle cx="40" cy="20" r="2" stroke="#d4af37" strokeWidth="1" opacity="0.5" />
      <circle cx="56" cy="32" r="2" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      <circle cx="56" cy="48" r="2" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <circle cx="40" cy="60" r="2" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <circle cx="24" cy="48" r="2" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <circle cx="24" cy="32" r="2" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      {/* Connections */}
      <line x1="40" y1="22" x2="54" y2="32" stroke="#d4af37" strokeWidth="0.5" opacity="0.3" />
      <line x1="56" y1="34" x2="56" y2="46" stroke="currentColor" strokeWidth="0.5" opacity="0.2" />
      <line x1="54" y1="48" x2="42" y2="58" stroke="currentColor" strokeWidth="0.5" opacity="0.2" />
      <line x1="38" y1="58" x2="26" y2="48" stroke="currentColor" strokeWidth="0.5" opacity="0.2" />
      <line x1="24" y1="46" x2="24" y2="34" stroke="#d4af37" strokeWidth="0.5" opacity="0.3" />
      <line x1="26" y1="32" x2="38" y2="22" stroke="#d4af37" strokeWidth="0.5" opacity="0.3" />
      {/* Center node */}
      <circle cx="40" cy="40" r="3" fill="#d4af37" opacity="0.15" />
      <circle cx="40" cy="40" r="1.5" fill="#d4af37" opacity="0.4" />
    </svg>
  );
}

export function NetworkIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Mesh network nodes */}
      <circle cx="40" cy="20" r="4" stroke="#d4af37" strokeWidth="1" opacity="0.5" />
      <circle cx="20" cy="40" r="4" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <circle cx="60" cy="40" r="4" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <circle cx="28" cy="60" r="4" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <circle cx="52" cy="60" r="4" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      {/* Connections */}
      <line x1="40" y1="24" x2="23" y2="37" stroke="currentColor" strokeWidth="0.5" opacity="0.2" />
      <line x1="40" y1="24" x2="57" y2="37" stroke="currentColor" strokeWidth="0.5" opacity="0.2" />
      <line x1="23" y1="43" x2="30" y2="57" stroke="currentColor" strokeWidth="0.5" opacity="0.2" />
      <line x1="57" y1="43" x2="50" y2="57" stroke="currentColor" strokeWidth="0.5" opacity="0.2" />
      <line x1="32" y1="60" x2="48" y2="60" stroke="#d4af37" strokeWidth="0.5" opacity="0.3" />
      <line x1="24" y1="40" x2="56" y2="40" stroke="currentColor" strokeWidth="0.5" opacity="0.15" strokeDasharray="2 3" />
      {/* Signal waves from top node */}
      <path d="M34 14C34 14 37 12 40 12C43 12 46 14 46 14" stroke="#d4af37" strokeWidth="0.5" opacity="0.3" />
      <path d="M32 10C32 10 36 8 40 8C44 8 48 10 48 10" stroke="#d4af37" strokeWidth="0.5" opacity="0.2" />
    </svg>
  );
}

export function OrdersIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Receipt */}
      <rect x="22" y="14" width="36" height="52" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      {/* Zigzag bottom edge */}
      <path d="M22 62L26 66L30 62L34 66L38 62L42 66L46 62L50 66L54 62L58 66" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      {/* Content lines */}
      <line x1="28" y1="24" x2="52" y2="24" stroke="currentColor" strokeWidth="1" opacity="0.15" />
      <line x1="28" y1="30" x2="46" y2="30" stroke="currentColor" strokeWidth="1" opacity="0.15" />
      <line x1="28" y1="38" x2="52" y2="38" stroke="currentColor" strokeWidth="0.5" opacity="0.1" />
      {/* Total line */}
      <line x1="28" y1="46" x2="52" y2="46" stroke="#d4af37" strokeWidth="1" opacity="0.3" />
      <line x1="38" y1="52" x2="52" y2="52" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
    </svg>
  );
}

export function DisputeIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Shield */}
      <path d="M40 16L60 26V42C60 54 52 62 40 66C28 62 20 54 20 42V26L40 16Z" stroke="currentColor" strokeWidth="1" opacity="0.3" fill="none" />
      {/* Checkmark */}
      <path d="M32 42L38 48L50 34" stroke="#d4af37" strokeWidth="1.5" opacity="0.5" />
    </svg>
  );
}

export function OffersIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Handshake / exchange */}
      <path d="M16 44H28L36 36L44 44L52 36" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <path d="M52 36H64" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      {/* Arrows going both ways */}
      <path d="M24 32L32 32M32 32L28 28M32 32L28 36" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <path d="M56 48L48 48M48 48L52 44M48 48L52 52" stroke="#d4af37" strokeWidth="1" opacity="0.3" />
      {/* Price tags */}
      <circle cx="28" cy="54" r="6" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <circle cx="52" cy="26" r="6" stroke="#d4af37" strokeWidth="1" opacity="0.3" />
    </svg>
  );
}

export function InvoiceIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Document */}
      <rect x="20" y="14" width="40" height="52" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      {/* Header line */}
      <rect x="28" y="22" width="24" height="4" rx="1" fill="#d4af37" opacity="0.15" />
      {/* Line items */}
      <line x1="28" y1="34" x2="52" y2="34" stroke="currentColor" strokeWidth="0.5" opacity="0.15" />
      <line x1="28" y1="40" x2="52" y2="40" stroke="currentColor" strokeWidth="0.5" opacity="0.15" />
      <line x1="28" y1="46" x2="52" y2="46" stroke="currentColor" strokeWidth="0.5" opacity="0.15" />
      {/* Divider */}
      <line x1="28" y1="52" x2="52" y2="52" stroke="#d4af37" strokeWidth="1" opacity="0.3" />
      {/* Total */}
      <rect x="38" y="56" width="14" height="4" rx="1" fill="#d4af37" opacity="0.2" />
    </svg>
  );
}

export function EscrowIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Lock box */}
      <rect x="20" y="32" width="40" height="28" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      {/* Lock */}
      <rect x="34" y="40" width="12" height="10" rx="1" stroke="#d4af37" strokeWidth="1" opacity="0.5" />
      <path d="M37 40V36C37 33.8 38.3 32 40 32C41.7 32 43 33.8 43 36V40" stroke="#d4af37" strokeWidth="1" opacity="0.5" fill="none" />
      <circle cx="40" cy="46" r="1.5" fill="#d4af37" opacity="0.4" />
      {/* Timer arc */}
      <path d="M28 24C28 18 33 14 40 14C47 14 52 18 52 24" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <line x1="40" y1="14" x2="40" y2="10" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <line x1="38" y1="10" x2="42" y2="10" stroke="currentColor" strokeWidth="1" opacity="0.2" />
    </svg>
  );
}

export function TransactionsIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Arrows up and down */}
      <path d="M30 20V54" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <path d="M30 20L26 26M30 20L34 26" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      <path d="M50 60V26" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <path d="M50 60L46 54M50 60L54 54" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      {/* Horizontal dashed lines */}
      <line x1="16" y1="36" x2="64" y2="36" stroke="currentColor" strokeWidth="0.5" opacity="0.1" strokeDasharray="2 4" />
      <line x1="16" y1="44" x2="64" y2="44" stroke="currentColor" strokeWidth="0.5" opacity="0.1" strokeDasharray="2 4" />
      {/* Gold dot */}
      <circle cx="30" cy="36" r="2" fill="#d4af37" opacity="0.3" />
      <circle cx="50" cy="44" r="2" fill="#d4af37" opacity="0.2" />
    </svg>
  );
}

export function DelegationIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Person silhouettes */}
      <circle cx="26" cy="28" r="6" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <path d="M16 46C16 40 20 36 26 36C32 36 36 40 36 46" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <circle cx="54" cy="28" r="6" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      <path d="M44 46C44 40 48 36 54 36C60 36 64 40 64 46" stroke="#d4af37" strokeWidth="1" opacity="0.3" />
      {/* Arrow from left to right */}
      <path d="M34 36L46 36" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      <path d="M44 33L47 36L44 39" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      {/* Power indicator */}
      <rect x="46" y="52" width="16" height="4" rx="1" fill="#d4af37" opacity="0.15" />
      <rect x="18" y="52" width="8" height="4" rx="1" fill="currentColor" opacity="0.1" />
    </svg>
  );
}

export function FollowingIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Person with + */}
      <circle cx="40" cy="30" r="10" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      <path d="M28 56C28 48 33 42 40 42C47 42 52 48 52 56" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      {/* Plus icon */}
      <line x1="56" y1="24" x2="56" y2="36" stroke="#d4af37" strokeWidth="1" opacity="0.5" />
      <line x1="50" y1="30" x2="62" y2="30" stroke="#d4af37" strokeWidth="1" opacity="0.5" />
      {/* Small dots representing a network */}
      <circle cx="18" cy="50" r="2" stroke="currentColor" strokeWidth="0.5" opacity="0.15" />
      <circle cx="62" cy="54" r="2" stroke="currentColor" strokeWidth="0.5" opacity="0.15" />
    </svg>
  );
}

export function ChatIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Chat bubble with cursor */}
      <rect x="12" y="20" width="56" height="40" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      {/* Typing cursor */}
      <rect x="20" y="36" width="2" height="12" fill="#d4af37" opacity="0.5" />
      {/* Ghost text lines */}
      <line x1="28" y1="38" x2="48" y2="38" stroke="currentColor" strokeWidth="1" opacity="0.08" strokeDasharray="2 3" />
      <line x1="28" y1="44" x2="42" y2="44" stroke="currentColor" strokeWidth="1" opacity="0.08" strokeDasharray="2 3" />
    </svg>
  );
}

export function ConversationsIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Stacked conversation bubbles */}
      <rect x="12" y="16" width="44" height="14" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.15" />
      <line x1="18" y1="23" x2="42" y2="23" stroke="currentColor" strokeWidth="0.5" opacity="0.1" />
      <rect x="12" y="34" width="44" height="14" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <line x1="18" y1="41" x2="38" y2="41" stroke="currentColor" strokeWidth="0.5" opacity="0.15" />
      <rect x="12" y="52" width="44" height="14" rx="2" stroke="#d4af37" strokeWidth="1" opacity="0.3" />
      <line x1="18" y1="59" x2="44" y2="59" stroke="#d4af37" strokeWidth="0.5" opacity="0.2" />
      {/* Time dots */}
      <circle cx="62" cy="23" r="1" fill="currentColor" opacity="0.15" />
      <circle cx="62" cy="41" r="1" fill="currentColor" opacity="0.2" />
      <circle cx="62" cy="59" r="1" fill="#d4af37" opacity="0.3" />
    </svg>
  );
}

export function CredentialIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Certificate body */}
      <rect x="14" y="18" width="52" height="40" rx="2" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      {/* Seal / badge circle */}
      <circle cx="40" cy="38" r="10" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      <circle cx="40" cy="38" r="6" stroke="#d4af37" strokeWidth="0.5" opacity="0.25" />
      {/* Checkmark inside seal */}
      <path d="M36 38L39 41L45 35" stroke="#d4af37" strokeWidth="1.5" opacity="0.5" />
      {/* Header line */}
      <line x1="24" y1="24" x2="56" y2="24" stroke="currentColor" strokeWidth="1" opacity="0.15" />
      {/* Footer lines — claim fields */}
      <line x1="24" y1="52" x2="40" y2="52" stroke="currentColor" strokeWidth="0.5" opacity="0.1" />
      <line x1="44" y1="52" x2="56" y2="52" stroke="#d4af37" strokeWidth="0.5" opacity="0.2" />
      {/* Ribbon tails */}
      <path d="M34 58V68L37 65L40 68V58" stroke="#d4af37" strokeWidth="1" opacity="0.3" />
      <path d="M40 58V68L43 65L46 68V58" stroke="#d4af37" strokeWidth="1" opacity="0.25" />
    </svg>
  );
}

export function IdentityKeyIllustration() {
  return (
    <svg width="80" height="80" viewBox="0 0 80 80" fill="none" xmlns="http://www.w3.org/2000/svg">
      {/* Key head — circle */}
      <circle cx="30" cy="32" r="12" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      <circle cx="30" cy="32" r="6" stroke="#d4af37" strokeWidth="0.5" opacity="0.25" />
      <circle cx="30" cy="32" r="2" fill="#d4af37" opacity="0.3" />
      {/* Key shaft */}
      <line x1="42" y1="32" x2="64" y2="32" stroke="currentColor" strokeWidth="1" opacity="0.3" />
      {/* Key teeth */}
      <line x1="56" y1="32" x2="56" y2="40" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      <line x1="62" y1="32" x2="62" y2="38" stroke="currentColor" strokeWidth="1" opacity="0.2" />
      {/* Fingerprint arcs */}
      <path d="M24 50C20 50 18 54 18 58C18 62 20 66 24 66" stroke="currentColor" strokeWidth="0.5" opacity="0.15" />
      <path d="M30 50C26 50 24 54 24 58C24 62 26 66 30 66" stroke="currentColor" strokeWidth="0.5" opacity="0.2" />
      <path d="M36 50C32 50 30 54 30 58C30 62 32 66 36 66" stroke="#d4af37" strokeWidth="0.5" opacity="0.3" />
      <path d="M42 52C38 52 36 55 36 58C36 61 38 64 42 64" stroke="#d4af37" strokeWidth="0.5" opacity="0.2" />
    </svg>
  );
}
