/**
 * Editorial 1px-stroke nav icons. 16×16 viewBox. currentColor.
 *
 * Drawn for Nous primary navigation — restrained, monochrome, no decorative
 * fills. They sit beside meta-cap labels in `--stone`, becoming `--oxblood`
 * when their route is active.
 *
 * If you find yourself reaching for a Lucide icon for the sidebar, the mobile
 * bottom-tab, or a hero shot, draw a new one here instead.
 */

import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement> & {
  size?: number;
};

function Icon({
  size = 16,
  strokeWidth = 1,
  children,
  ...rest
}: IconProps & { children: React.ReactNode }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth as number}
      strokeLinecap="square"
      strokeLinejoin="miter"
      aria-hidden="true"
      {...rest}
    >
      {children}
    </svg>
  );
}

export function IconDashboard(p: IconProps) {
  return (
    <Icon {...p}>
      <rect x="2" y="2" width="5" height="6" />
      <rect x="9" y="2" width="5" height="3" />
      <rect x="2" y="10" width="5" height="4" />
      <rect x="9" y="7" width="5" height="7" />
    </Icon>
  );
}

export function IconSocial(p: IconProps) {
  return (
    <Icon {...p}>
      <circle cx="6" cy="6" r="2.4" />
      <circle cx="11" cy="9" r="1.8" />
      <path d="M2 14c0-2 2-3.5 4-3.5s4 1.5 4 3.5" />
      <path d="M9 14c0-1.4 1.4-2.5 2.5-2.5s2.5 1 2.5 2.5" />
    </Icon>
  );
}

export function IconMessages(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M2 3h12v8H6l-3 2.5V11H2z" />
    </Icon>
  );
}

export function IconWallet(p: IconProps) {
  return (
    <Icon {...p}>
      <rect x="2" y="4" width="12" height="9" />
      <path d="M2 6h12" />
      <circle cx="11.5" cy="9.5" r="0.6" fill="currentColor" stroke="none" />
    </Icon>
  );
}

export function IconMarketplace(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M2 5l1-2h10l1 2" />
      <path d="M2 5h12v8H2z" />
      <path d="M2 5c0 1.2.9 2 2 2s2-0.8 2-2" />
      <path d="M6 5c0 1.2.9 2 2 2s2-0.8 2-2" />
      <path d="M10 5c0 1.2.9 2 2 2s2-0.8 2-2" />
    </Icon>
  );
}

export function IconGovernance(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M2 13h12" />
      <path d="M3 13V7" />
      <path d="M13 13V7" />
      <path d="M6 13V7" />
      <path d="M10 13V7" />
      <path d="M2 7h12" />
      <path d="M2 7l6-4 6 4" />
    </Icon>
  );
}

export function IconAi(p: IconProps) {
  return (
    <Icon {...p}>
      <rect x="3" y="4" width="10" height="9" />
      <path d="M6 4V2" />
      <path d="M10 4V2" />
      <path d="M3 8H1" />
      <path d="M15 8h-2" />
      <circle cx="6.5" cy="8" r="0.6" fill="currentColor" stroke="none" />
      <circle cx="9.5" cy="8" r="0.6" fill="currentColor" stroke="none" />
      <path d="M6 11h4" />
    </Icon>
  );
}

export function IconIdentity(p: IconProps) {
  return (
    <Icon {...p}>
      <circle cx="8" cy="6" r="2.4" />
      <path d="M3 14c0-2.5 2.2-4.5 5-4.5s5 2 5 4.5" />
    </Icon>
  );
}

export function IconNetwork(p: IconProps) {
  return (
    <Icon {...p}>
      <circle cx="8" cy="8" r="6" />
      <path d="M2 8h12" />
      <path d="M8 2c2.2 2 3.4 4.2 3.4 6s-1.2 4-3.4 6c-2.2-2-3.4-4.2-3.4-6s1.2-4 3.4-6z" />
    </Icon>
  );
}

export function IconFiles(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M2 4h4l1 1.5h7V13H2z" />
    </Icon>
  );
}

export function IconSettings(p: IconProps) {
  return (
    <Icon {...p}>
      <circle cx="8" cy="8" r="2" />
      <path d="M8 1v2.5" />
      <path d="M8 12.5V15" />
      <path d="M1 8h2.5" />
      <path d="M12.5 8H15" />
      <path d="M3 3l1.8 1.8" />
      <path d="M11.2 11.2L13 13" />
      <path d="M3 13l1.8-1.8" />
      <path d="M11.2 4.8L13 3" />
    </Icon>
  );
}

export function IconSearch(p: IconProps) {
  return (
    <Icon {...p}>
      <circle cx="7" cy="7" r="4.5" />
      <path d="M10.5 10.5L14 14" />
    </Icon>
  );
}

export function IconArrow(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M3 8h10" />
      <path d="M9 4l4 4-4 4" />
    </Icon>
  );
}

export function IconChevronDown(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M3 6l5 5 5-5" />
    </Icon>
  );
}

export function IconClose(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M3 3l10 10" />
      <path d="M13 3L3 13" />
    </Icon>
  );
}

export function IconMenu(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M2 4h12" />
      <path d="M2 8h12" />
      <path d="M2 12h12" />
    </Icon>
  );
}
