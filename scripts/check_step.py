# Contrôle d'un fichier STEP exporté par FabVents, dans FreeCAD (OpenCascade).
#
#   STEP=chemin/fichier.step /Applications/FreeCAD.app/Contents/Resources/bin/freecadcmd scripts/check_step.py
#
# Affiche une ligne RESULT : nombre de solides, volume (mm³), solides
# invalides (contrôle individuel de chaque solide), types de faces, boîte
# englobante. Plusieurs dalles d'un même fichier se touchent le long des
# coupes : le contrôle booléen global les signale, d'où le contrôle par solide.
import collections
import os
import traceback

import Part

try:
    path = os.environ["STEP"]
    shape = Part.Shape()
    shape.read(path)
    bad = []
    for i, s in enumerate(shape.Solids):
        try:
            s.check(True)
        except Exception as e:  # noqa: BLE001 - message d'OpenCascade
            bad.append((i, str(e)[:120]))
    faces = collections.Counter(f.Surface.__class__.__name__ for f in shape.Faces)
    bb = shape.BoundBox
    print(
        "RESULT", os.path.basename(path),
        "solides", len(shape.Solids),
        "volume %.4f" % shape.Volume,
        "invalides", len(bad), bad[:3],
        "faces", dict(faces),
        "bbox", [round(v, 3) for v in (bb.XMin, bb.YMin, bb.ZMin, bb.XMax, bb.YMax, bb.ZMax)],
    )
except Exception:
    print("RESULT ERREUR", traceback.format_exc())
