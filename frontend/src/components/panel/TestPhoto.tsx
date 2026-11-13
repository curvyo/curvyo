import { Camera, Crop, Trash2, TriangleAlert, Upload } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

import { PhotoCrop } from "@/components/panel/PhotoCrop";
import { ARM_TRASH, ARM_ZONE, type DeleteArm } from "@/hooks/useDeleteArm";
import { useCamera } from "@/hooks/useCamera";
import type { MaterialsApi } from "@/hooks/useMaterials";
import type { TestDetail } from "@/lib/materialsView";
import { photoArmText } from "@/lib/materialRows";
import { PHOTO_MAX_PX, encodeType, isPhotoFile, scaledSize, type Frame } from "@/lib/photoSize";

const OUTLINE_BUTTON =
  "flex h-7 items-center gap-1 rounded border border-[var(--lane-line)] px-2 text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** What the photo section shows: the stored image, the live camera or Crop. */
type Mode = { kind: "view" } | { kind: "camera" } | { kind: "crop"; url: string; width: number; height: number; mime: string };

/** A canvas as encoded bytes of `mime` (JPEG at 0.85, PNG as is). */
async function encode(canvas: HTMLCanvasElement, mime: string, quality: number | undefined): Promise<Uint8Array | null> {
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, mime, quality));
  return blob ? new Uint8Array(await blob.arrayBuffer()) : null;
}

/** A source scaled to at most 600 px on its long side, drawn on a fresh canvas. */
function scaledCanvas(source: CanvasImageSource, width: number, height: number): HTMLCanvasElement {
  const size = scaledSize(width, height, PHOTO_MAX_PX);
  const canvas = document.createElement("canvas");
  canvas.width = size.width;
  canvas.height = size.height;
  canvas.getContext("2d")?.drawImage(source, 0, 0, size.width, size.height);
  return canvas;
}

interface TestPhotoProps {
  material: string;
  test: TestDetail;
  api: MaterialsApi;
  arm: DeleteArm;
}

/**
 * The photo of a test (`specs/0112` criteria 27, 31; `ux-notes.md` 8.5): the stored image with its
 * armed trash, then File, Camera (only with a camera and a permission that is not refused) and
 * Crop. A photo from a file or the camera is scaled to 600 px on its long side, stored and opened
 * in Crop at once; Crop writes the cut image. A missing or damaged file shows "Photo not found".
 */
export function TestPhoto({ material, test, api, arm }: TestPhotoProps) {
  const [mode, setMode] = useState<Mode>({ kind: "view" });
  const [version, setVersion] = useState(0);
  const [loaded, setLoaded] = useState<{ name: string; version: number; url: string | null } | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  // Crop and the live view replace the buttons; leaving them, the focus goes back to File.
  const fileButton = useRef<HTMLButtonElement>(null);
  const refocus = useRef(false);
  useEffect(() => {
    if (mode.kind === "view" && refocus.current) {
      refocus.current = false;
      fileButton.current?.focus();
    }
  }, [mode.kind]);
  const toView = () => {
    refocus.current = true;
    setMode({ kind: "view" });
  };
  const { offered: cameraOffered, attach: attachVideo, video: videoElement, start: startStream, stop: stopStream } = useCamera();
  const { photo } = api;
  const name = test.photo;

  useEffect(() => {
    if (name === null) return undefined;
    let alive = true;
    let url: string | null = null;
    void photo(name).then((bytes) => {
      if (!alive) return;
      if (bytes !== "missing") url = URL.createObjectURL(new Blob([bytes.slice().buffer]));
      setLoaded({ name, version, url });
    });
    return () => {
      alive = false;
      if (url !== null) URL.revokeObjectURL(url);
    };
  }, [name, version, photo]);

  // The crop's object URL is revoked when Crop closes.
  const cropUrl = mode.kind === "crop" ? mode.url : null;
  useEffect(() => {
    return () => {
      if (cropUrl !== null) URL.revokeObjectURL(cropUrl);
    };
  }, [cropUrl]);

  const store = useCallback(
    async (canvas: HTMLCanvasElement, mime: string, cropped: boolean): Promise<boolean> => {
      const type = encodeType(mime);
      const bytes = await encode(canvas, type.mime, type.quality);
      if (bytes === null) {
        api.showNotice("Could not read the image.", true);
        return false;
      }
      const refused = api.setPhoto(material, test.id, bytes, cropped);
      setVersion((n) => n + 1);
      return refused === "";
    },
    [api, material, test.id],
  );

  const openCrop = async (canvas: HTMLCanvasElement, mime: string) => {
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, mime === "image/png" ? "image/png" : "image/jpeg", 0.85));
    if (!blob) return;
    setMode({ kind: "crop", url: URL.createObjectURL(blob), width: canvas.width, height: canvas.height, mime });
  };

  const onFile = async (file: File | undefined) => {
    if (!file) return;
    if (!isPhotoFile(file.type)) {
      api.showNotice("Use a JPEG or PNG image.", true);
      return;
    }
    try {
      const bitmap = await createImageBitmap(file);
      const canvas = scaledCanvas(bitmap, bitmap.width, bitmap.height);
      bitmap.close();
      if (await store(canvas, file.type, false)) await openCrop(canvas, file.type);
    } catch {
      api.showNotice("Use a JPEG or PNG image.", true);
    }
  };

  const startCamera = async () => {
    const refusal = await startStream();
    if (refusal === null) setMode({ kind: "camera" });
    else api.showNotice(refusal, true);
  };
  const cancelCamera = () => {
    stopStream();
    toView();
  };
  const takePhoto = async () => {
    const video = videoElement();
    if (!video || video.videoWidth === 0) return;
    const canvas = scaledCanvas(video, video.videoWidth, video.videoHeight);
    stopStream();
    setMode({ kind: "view" });
    if (await store(canvas, "image/jpeg", false)) await openCrop(canvas, "image/jpeg");
  };

  const crop = async (frame: Frame) => {
    if (mode.kind !== "crop") return;
    const image = new Image();
    image.src = mode.url;
    try {
      await image.decode();
    } catch {
      setMode({ kind: "view" });
      return;
    }
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(frame.width));
    canvas.height = Math.max(1, Math.round(frame.height));
    canvas.getContext("2d")?.drawImage(image, frame.x, frame.y, frame.width, frame.height, 0, 0, canvas.width, canvas.height);
    const mime = mode.mime;
    toView();
    await store(canvas, mime, true);
  };

  const openCropOfStored = (url: string) => {
    const image = new Image();
    image.onload = () => {
      // Crop works on its own copy of the stored image.
      void fetch(url)
        .then((response) => response.blob())
        .then((blob) => setMode({ kind: "crop", url: URL.createObjectURL(blob), width: image.naturalWidth, height: image.naturalHeight, mime: blob.type }));
    };
    image.src = url;
  };

  const stored = name !== null && loaded !== null && loaded.name === name && loaded.version === version ? loaded : null;
  const key = `photo:${material}:${test.id}`;
  const armed = arm.armed === key;

  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs font-semibold text-[var(--toolbar-icon)]">Photo</span>
      {mode.kind === "crop" ? (
        <PhotoCrop url={mode.url} width={mode.width} height={mode.height} onCancel={toView} onCrop={(frame) => void crop(frame)} />
      ) : mode.kind === "camera" ? (
        <div
          className="flex flex-col gap-1"
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              event.stopPropagation();
              cancelCamera();
            }
          }}
        >
          <video ref={attachVideo} muted playsInline aria-label="Camera" className="w-[228px] bg-[var(--canvas-bg)]" />
          <div className="flex gap-1">
            <button type="button" onClick={cancelCamera} className={`${OUTLINE_BUTTON} w-[112px] justify-center`}>
              Cancel
            </button>
            <button
              type="button"
              autoFocus
              onClick={() => void takePhoto()}
              className="flex h-7 w-[112px] items-center justify-center gap-1 rounded bg-[var(--toolbar-icon-active-bg)] text-xs text-white focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
            >
              <Camera size={16} aria-hidden />
              Take photo
            </button>
          </div>
        </div>
      ) : (
        <>
          {name !== null ? (
            <div className="relative w-[228px]" {...{ [ARM_ZONE]: key }}>
              {stored === null ? (
                <div className="h-7" aria-hidden />
              ) : stored.url === null ? (
                <p className="flex items-center gap-1 text-xs text-[var(--panel-muted-fg)]">
                  <TriangleAlert size={16} aria-hidden className="shrink-0" />
                  Photo not found
                </p>
              ) : (
                <img src={stored.url} alt={`Photo of test ${test.id}`} className="max-h-[228px] w-[228px] bg-[var(--canvas-bg)] object-contain" />
              )}
              <button
                type="button"
                {...{ [ARM_TRASH]: key }}
                aria-label={armed ? photoArmText() : "Remove photo"}
                title={armed ? "Press again to remove" : "Remove photo"}
                onClick={() =>
                  arm.press(key, photoArmText(), () => {
                    api.removePhoto(material, test.id);
                    return "Photo removed.";
                  })
                }
                className={`absolute right-1 top-1 flex size-[22px] items-center justify-center rounded-[5px] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
                  armed ? "bg-[var(--destructive)] text-white" : "bg-[var(--toolbar-bg)] text-[var(--toolbar-icon)]"
                }`}
              >
                <Trash2 size={16} aria-hidden />
              </button>
            </div>
          ) : null}
          <div className="flex flex-wrap gap-1">
            <input
              ref={fileInput}
              type="file"
              accept="image/jpeg,image/png"
              hidden
              onChange={(event) => {
                void onFile(event.target.files?.[0]);
                event.target.value = "";
              }}
            />
            <button ref={fileButton} type="button" onClick={() => fileInput.current?.click()} className={OUTLINE_BUTTON}>
              <Upload size={16} aria-hidden />
              File
            </button>
            {cameraOffered ? (
              <button type="button" onClick={() => void startCamera()} className={OUTLINE_BUTTON}>
                <Camera size={16} aria-hidden />
                Camera
              </button>
            ) : null}
            {stored?.url ? (
              <button type="button" onClick={() => openCropOfStored(stored.url as string)} className={OUTLINE_BUTTON}>
                <Crop size={16} aria-hidden />
                Crop
              </button>
            ) : null}
          </div>
        </>
      )}
    </div>
  );
}
