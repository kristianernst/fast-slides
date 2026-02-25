"use client";

import { Cross2Icon } from "@radix-ui/react-icons";

type ExpandableAsset = {
  kind: "image" | "video";
  src: string;
  alt: string;
};

type AssetLightboxProps = {
  asset: ExpandableAsset | null;
  onClose: () => void;
};

export function AssetLightbox({ asset, onClose }: AssetLightboxProps) {
  if (!asset) {
    return null;
  }

  return (
    <div
      className="asset-lightbox-overlay"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-label="Expanded slide asset"
    >
      <button
        type="button"
        className="asset-lightbox-close"
        onClick={onClose}
        aria-label="Close expanded asset"
      >
        <Cross2Icon aria-hidden="true" />
      </button>
      <div className="asset-lightbox-content" onClick={(event) => event.stopPropagation()}>
        {asset.kind === "image" ? (
          <img src={asset.src} alt={asset.alt || "Expanded slide image"} />
        ) : (
          <video src={asset.src} controls autoPlay playsInline />
        )}
        {asset.alt ? <p className="asset-lightbox-caption">{asset.alt}</p> : null}
      </div>
    </div>
  );
}
