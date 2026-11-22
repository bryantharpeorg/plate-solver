import type { ObservingSession } from "../api";

const reasons = {
  only: "",
  first: "the first given; nothing has solved yet",
  field: "the solved field lies in its range",
  solved: "it solved the last frame; its field lies in no range",
};

export function FieldWarning({ field }: { field: ObservingSession["field"] }) {
  if (!field) return null;
  const { fov_deg: fov, db_min_fov_deg: min, db_max_fov_deg: max, unsolved_s, database, databases = [] } = field;
  const range = `${min}–${max}°`;
  const several = databases.length > 1;
  const ranges = databases.map((db) => `${db.name} ${db.min_fov_deg}–${db.max_fov_deg}°`).join(", ");
  const nearEdge = fov !== null && (fov <= min + 1 || fov >= max - 1);
  let warning: string | null = null;
  if (unsolved_s >= 30) {
    warning = several
      ? `Frames with at least 10 stars have not solved for 30 s with any database (${ranges}). The camera's field may be outside every range; a database built for the camera's field may be needed.`
      : `Frames with at least 10 stars have not solved for 30 s. The camera's field may be outside the database's ${range} range; a matching database may be needed.`;
  } else if (several && fov !== null && !databases.some((db) => fov >= db.min_fov_deg && fov <= db.max_fov_deg)) {
    warning = `The solved field, ${fov}°, is outside every database's range (${ranges}). A database built for the camera's field may be needed.`;
  } else if (!several && nearEdge) {
    warning =
      "The solved field is within 1° of an end of the database's range or outside it. A database built for the camera's field may be needed.";
  }
  return (
    <div className="space-y-2 border-y border-rule py-3">
      <p>Camera field: {fov === null ? "not solved yet" : `${fov}°`}</p>
      <p className="quiet">Database range: {range}</p>
      {several && (
        <>
          {database && (
            <p className="quiet">
              Database in use: {database.name}, {reasons[database.reason]}
            </p>
          )}
          <p className="quiet">Databases: {ranges}</p>
        </>
      )}
      {warning && (
        <p role="alert" className="text-ink">
          {warning}
        </p>
      )}
    </div>
  );
}
