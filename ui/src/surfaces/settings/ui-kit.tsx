import { useState } from "react";
import {
  ActionTile, Alert, Badge, Button, Card, Checkbox, ConfirmDialog, Dialog, Disclosure,
  EmptyState, HoverTip, IconTile, Input, LoadingButton, PasswordInput, Progress,
  Radio, SearchField, Section, Select, SettingRow, Slider, SurfaceButton, Switch, Tabs, TextField,
} from "../../meridian-ui";
import { useAppearance } from "../../lib/appearance";

export function UIKit() {
  const mode = useAppearance();
  const [checked, setChecked] = useState(true), [level, setLevel] = useState(40);
  const [search, setSearch] = useState(""), [selection, setSelection] = useState("one");
  const [tab, setTab] = useState("first"), [dialog, setDialog] = useState(false), [confirm, setConfirm] = useState(false);
  const [message, setMessage] = useState("Ready for preview");
  return <>
    <Alert heading={`Previewing ${mode} theme`}>Use Appearance → Color theme to inspect the other palette. These controls change only this preview. Use Tab, arrow keys and Escape to check keyboard behavior.</Alert>
    <div className="muk-preview-grid">
      <Card className="muk-preview-group"><h3>Buttons and feedback</h3>
        <div className="muk-preview-actions">{(["default", "primary", "secondary", "danger", "ghost"] as const).map(variant => <Button key={variant} variant={variant} onClick={() => setMessage(`${variant} button activated`)}>{variant}</Button>)}</div>
        <div className="muk-preview-actions"><Button disabled>Disabled</Button><LoadingButton loading>Save</LoadingButton><SurfaceButton onClick={() => setMessage("Surface action activated")}>Surface action</SurfaceButton><HoverTip label="Optional keyboard and mouse hint"><Button>Hint</Button></HoverTip></div>
        <p role="status">{message}</p>
        {(["info", "success", "warning", "error"] as const).map(tone => <Alert key={tone} tone={tone} heading={tone}>Example {tone} message</Alert>)}
      </Card>
      <Card className="muk-preview-group"><h3>Fields</h3>
        <TextField label="Display name" help="A labeled field with supporting text" placeholder="Name" />
        <TextField label="Invalid value" defaultValue="?" error="Enter a valid value." />
        <TextField label="Unavailable field" disabled value="Disabled" />
        <PasswordInput label="Example password" autoComplete="new-password" help="Use sample text here." />
        <Input aria-label="Basic input" placeholder="Basic input" />
        <SearchField aria-label="Preview search" value={search} onValueChange={setSearch} placeholder="Search preview" />
        <label>Choice<Select value={selection} onChange={event => setSelection(event.target.value)}><option value="one">First option</option><option value="two">Second option with a longer label</option><option disabled value="three">Unavailable</option></Select></label>
      </Card>
      <Card className="muk-preview-group"><h3>Selection and progress</h3>
        <Switch label="Example switch" checked={checked} onChange={setChecked} />
        <Switch label="Unavailable switch" checked={false} onChange={() => {}} disabled />
        <label><Checkbox checked={checked} onChange={event => setChecked(event.target.checked)} /> Checkbox</label>
        <label><Radio name="kit-radio" value="one" defaultChecked /> First radio</label>
        <label><Radio name="kit-radio" value="two" /> Second radio</label>
        <Slider aria-label="Example slider" value={level} onValueChange={setLevel} /><output>{level}%</output>
        <Slider aria-label="Unavailable slider" value={30} disabled />
        <Progress aria-label="Determinate progress" value={level} max={100} />
        <Progress aria-label="Indeterminate progress" />
        <div className="muk-preview-actions">{(["green", "blue", "orange", "red", "white"] as const).map(tone => <span key={tone}><IconTile icon="info" tone={tone} /><Badge tone={tone}>{tone}</Badge></span>)}</div>
      </Card>
      <Card className="muk-preview-group"><h3>Navigation and dialogs</h3>
        <ActionTile title="A navigation tile with a long title" description="Description wraps to fit narrow screens." icon="accessibility" onClick={() => setMessage("Navigation tile activated")} />
        <ActionTile title="Unavailable tile" icon="settings" disabled />
        <Tabs label="Example tabs" value={tab} onValueChange={setTab} items={[{value:"first",label:"First"},{value:"second",label:"Second"},{value:"disabled",label:"Unavailable",disabled:true}]}><p>{tab} tab content</p></Tabs>
        <Disclosure title="Expandable content"><p>Native keyboard-accessible disclosure content.</p></Disclosure>
        <EmptyState title="No results" description="Try a different search." action={<Button onClick={() => setSearch("")}>Clear preview search</Button>} />
        <Button onClick={() => setDialog(true)}>Open dialog</Button><Button onClick={() => setConfirm(true)}>Open confirmation</Button>
      </Card>
    </div>
    <Section title="Grouped settings" description="Section and SettingRow composition"><SettingRow title="Example preference" description="The label and switch are associated by their accessible names."><Switch label="Example preference" checked={checked} onChange={setChecked} /></SettingRow></Section>
    <Dialog open={dialog} onClose={() => setDialog(false)} aria-labelledby="kit-dialog-title"><h2 id="kit-dialog-title">Preview dialog</h2><p>Tab stays inside this dialog. Escape returns focus to its trigger.</p><Button onClick={() => setDialog(false)}>Close</Button></Dialog>
    <ConfirmDialog open={confirm} title="Confirm preview action?" onClose={() => setConfirm(false)} onConfirm={async () => setMessage("Preview action confirmed")}>This updates the preview status message.</ConfirmDialog>
  </>;
}
