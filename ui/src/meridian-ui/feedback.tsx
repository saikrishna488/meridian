import type { ComponentProps, ReactNode } from "react";
import { Icon, type IconName } from "../components/Icon";
import { Button, type ButtonProps } from "../components/ui/button";
import { IconTile } from "./patterns";

export type AlertProps = ComponentProps<"div"> & {
  tone?: "info" | "success" | "warning" | "error";
  heading?: string;
  action?: ReactNode;
};
export function Alert({ tone = "info", heading, action, children, className = "", role, ...props }: AlertProps) {
  return <div {...props} role={role ?? (tone === "error" ? "alert" : "status")} className={`muk-alert ${className}`} data-tone={tone}>
    <Icon name={tone === "success" ? "check" : "info"} />
    <div className="muk-alert-copy">{heading && <strong>{heading}</strong>}{children}</div>
    {action && <div className="muk-alert-action">{action}</div>}
  </div>;
}

export type DisclosureProps = Omit<ComponentProps<"details">, "title"> & { title: string };
/** Native details preserves keyboard activation and controlled/uncontrolled open state. */
export function Disclosure({ title, children, className = "", ...props }: DisclosureProps) {
  return <details {...props} className={`muk-disclosure ${className}`}>
    <summary>{title}</summary><div className="muk-disclosure-body">{children}</div>
  </details>;
}

export type ActionTileProps = Omit<ButtonProps, "children" | "title"> & {
  title: string;
  description?: string;
  icon: IconName;
};
export function ActionTile({ title, description, icon, className = "", ...props }: ActionTileProps) {
  return <Button {...props} className={`muk-action-tile ${className}`}>
    <IconTile icon={icon} />
    <span className="muk-action-tile-copy"><strong>{title}</strong>{description && <small>{description}</small>}</span>
    <Icon name="forward" width="16" height="16" />
  </Button>;
}
