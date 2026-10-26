// The Info mode of the font preview pane (`specs/0083-font-info-and-zip` criteria 1 to 6, UX
// notes §2, §7 and §9): the subject face's family and style drawn in the face, then the facts
// as label and value rows in the order of criterion 2. One scroll area, focusable; the content
// swaps with the subject face and scrolls back to the top.
import { useEffect, useRef, useState } from "react";

import { InfoDocuments } from "@/components/fontPreview/InfoDocuments";
import { InfoValue } from "@/components/fontPreview/InfoValue";
import { LineName } from "@/components/panel/FontTreeRows";
import type { FontInfoState } from "@/hooks/useFontInfo";
import type { GlyphSubject } from "@/hooks/useText";
import { cssFamilyOf, linePreviewOf, loadFace } from "@/lib/fontFaceLoader";
import {
  blockText,
  countText,
  embeddingText,
  fileLine,
  formatText,
  licenceName,
  missingInfoText,
  unreadableInfoText,
} from "@/lib/fontInfoText";
import { isTauri } from "@/platform/host";

interface InfoViewProps {
  subject: GlyphSubject | null;
  info: FontInfoState;
  full: boolean;
}

function Row({ label, gap, children }: { label: string; gap?: boolean; children: React.ReactNode }) {
  return (
    <div className={`grid grid-cols-[96px_minmax(0,1fr)] gap-x-3 ${gap ? "mt-4" : "mt-2"}`}>
      <dt className="text-sm text-[var(--panel-muted-fg)]">{label}</dt>
      {children}
    </div>
  );
}

function Header({ subject, info }: { subject: GlyphSubject; info: FontInfoState }) {
  const [, setLoaded] = useState(0);
  const face = subject.face;
  useEffect(() => {
    if (face && !subject.missing) {
      loadFace(face, () => setLoaded((n) => n + 1));
    }
  }, [face, subject.missing]);
  const css = face && info.status === "ready" ? cssFamilyOf(face.id) : undefined;
  const lines = face?.singleLine ? linePreviewOf(face.id) : undefined;
  const style = face?.style ?? "";
  return (
    <div className="min-w-0">
      {lines?.family ? (
        <div className="overflow-hidden">
          <span className="sr-only">{subject.family}</span>
          <LineName preview={lines.family} em={28} />
        </div>
      ) : (
        <p className="m-0 truncate text-[28px] leading-tight text-[var(--toolbar-icon)]" style={css ? { fontFamily: css } : undefined}>
          {subject.family}
        </p>
      )}
      {style && !subject.missing ? (
        <p
          className="m-0 truncate text-base text-[var(--panel-muted-fg)]"
          style={css && !lines ? { fontFamily: css } : undefined}
        >
          {style}
        </p>
      ) : null}
    </div>
  );
}

export function InfoView({ subject, info, full }: InfoViewProps) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const faceKey = subject?.face ? `${subject.face.id}#${subject.face.faceIndex}` : (subject?.family ?? "");
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 });
  }, [faceKey]);

  const fields = info.fields;
  const face = subject?.face ?? null;
  const singleLine = fields?.format.kind === "svg";
  let body: React.ReactNode = null;
  if (subject && info.status === "missing") {
    body = <p className="m-0 mt-4 text-sm text-[var(--panel-muted-fg)]">{missingInfoText(subject.family)}</p>;
  } else if (subject && info.status === "unreadable") {
    body = <p className="m-0 mt-4 text-sm text-[var(--panel-muted-fg)]">{unreadableInfoText(subject.name)}</p>;
  } else if (face && fields) {
    const licence = info.licence;
    const licenceText = licence ? licenceName(licence.id) : fields.licence;
    body = (
      <dl key={faceKey} className="m-0 mt-4 max-w-[640px]">
        <Row label="Full name">
          <InfoValue text={fields.full_name} />
        </Row>
        <Row label="Format" gap>
          <InfoValue text={formatText(fields.format)} clamp={false} />
        </Row>
        <Row label="File">
          <dd className="m-0 min-w-0 text-sm text-[var(--toolbar-icon)] select-text [overflow-wrap:anywhere]">
            <p className="m-0">{fileLine(info.file, info.size)}</p>
            <p className="m-0">{blockText(face.block, isTauri)}</p>
            {info.path ? <p className="m-0 text-xs text-[var(--panel-muted-fg)]">{info.path}</p> : null}
          </dd>
        </Row>
        <Row label="Version" gap>
          <InfoValue text={fields.version} />
        </Row>
        <Row label="Designer">
          <InfoValue text={fields.designer} />
        </Row>
        <Row label="Vendor">
          <InfoValue text={fields.vendor} />
        </Row>
        <Row label="Copyright">
          <InfoValue text={fields.copyright} />
        </Row>
        <Row label="Trademark">
          <InfoValue text={fields.trademark} />
        </Row>
        <Row label="Licence" gap>
          <dd className="m-0 flex min-w-0 flex-col gap-1">
            <InfoValue part text={licenceText} />
            {licence?.text ? <InfoValue part text={licence.text} /> : null}
            {!licence && fields.licence_url ? <InfoValue part text={fields.licence_url} clamp={false} /> : null}
          </dd>
        </Row>
        <Row label="Embedding" gap>
          <InfoValue text={embeddingText(fields.fs_type, singleLine)} clamp={false} />
        </Row>
        <Row label="Glyphs">
          <InfoValue text={countText(fields.glyphs, "glyph")} clamp={false} />
        </Row>
        <Row label="Characters">
          <InfoValue text={countText(fields.characters, "character")} clamp={false} />
        </Row>
        {info.folder ? (
          <Row label="Documents" gap>
            <InfoDocuments key={faceKey} folder={info.folder.name} documents={info.folder.documents} full={full} />
          </Row>
        ) : null}
      </dl>
    );
  }

  return (
    <div
      ref={scrollRef}
      role="region"
      aria-label="Font info"
      tabIndex={0}
      className="min-h-0 flex-1 overflow-y-auto px-4 py-4 outline-none [overscroll-behavior:contain] [scrollbar-width:thin] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]"
    >
      {subject ? <Header subject={subject} info={info} /> : null}
      {body}
    </div>
  );
}
