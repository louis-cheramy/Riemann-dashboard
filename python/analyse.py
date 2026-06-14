import struct
import os
import numpy as np
import streamlit as st
import matplotlib.pyplot as plt
import riemann_viz
MAGIC = b"PRIMEV2\x00"

def lire_nombres_premiers(fichier):
    if not os.path.exists(fichier):
        st.error(f"Fichier '{fichier}' introuvable.")
        return np.array([], dtype=np.uint64)
    with open(fichier, "rb") as f:
        header = f.read(8)
        if header == MAGIC:
            return np.frombuffer(f.read(), dtype="<u8")
        f.seek(0)
        data = f.read()
    if len(data) % 4 != 0:
        st.error("Format de fichier invalide.")
        return np.array([], dtype=np.uint64)
    count = len(data) // 4
    return np.array(struct.unpack(f"<{count}I", data), dtype=np.uint64)

# Lecture des nombres premiers
data_file = "nombres_premiers.bin"
nombres_premiers = lire_nombres_premiers(data_file)

st.title("Analyse des Nombres Premiers")

if nombres_premiers.size == 0:
    st.stop()

st.write(f"Nombre total de nombres premiers chargés : {len(nombres_premiers):,}")

# Sélection de l'intervalle

defaut_min = int(nombres_premiers[0])
defaut_max = int(nombres_premiers[-1])

col1, col2 = st.columns(2)
with col1:
    borne_min = st.number_input("Borne minimale de l'intervalle", min_value=defaut_min, max_value=defaut_max, value=defaut_min, step=1)
with col2:
    borne_max = st.number_input("Borne maximale de l'intervalle", min_value=defaut_min, max_value=defaut_max, value=min(defaut_max, defaut_min + 100_000), step=1)

# Slider pour visualiser l'intervalle (optionnel, synchronisé)
intervalle = st.slider(
    "(Optionnel) Ajustez l'intervalle avec le curseur",
    min_value=defaut_min,
    max_value=defaut_max,
    value=(borne_min, borne_max),
    step=1
)
# Synchronisation : si le slider change, on met à jour les champs (Streamlit ne permet pas de synchroniser dynamiquement, donc priorité aux champs number_input)
borne_min, borne_max = borne_min, borne_max
if intervalle != (borne_min, borne_max):
    borne_min, borne_max = intervalle

selection = nombres_premiers[(nombres_premiers >= borne_min) & (nombres_premiers <= borne_max)]

st.write(f"Nombres premiers dans l'intervalle [{borne_min:,} ; {borne_max:,}] : {len(selection):,}")

# Choix du type de graphe
graphe = st.selectbox(
    "Type de graphe",
    [
        "Histogramme (répartition)",
        "Espacement entre premiers",
        "Zéros de la fonction zêta de Riemann",
        "Affichage des entiers et des nombres premiers"
    ]
)

if graphe == "Histogramme (répartition)":
    nb_bins = st.slider("Nombre de classes (bins)", 10, 200, 50)
    fig, ax = plt.subplots()
    ax.hist(selection, bins=nb_bins, color='royalblue', edgecolor='black')
    ax.set_title("Répartition des nombres premiers")
    ax.set_xlabel("Valeur")
    ax.set_ylabel("Nombre de premiers")
    st.pyplot(fig)
elif graphe == "Espacement entre premiers":
    if len(selection) < 2:
        st.warning("Intervalle trop petit pour calculer les espacements.")
    else:
        espacements = np.diff(selection)
        fig, ax = plt.subplots()
        ax.hist(espacements, bins=30, color='orange', edgecolor='black')
        ax.set_title("Distribution des espacements entre nombres premiers")
        ax.set_xlabel("Espacement")
        ax.set_ylabel("Fréquence")
        st.pyplot(fig)
elif graphe == "Zéros de la fonction zêta de Riemann":
    st.markdown("""
    **Zéros triviaux** : entiers pairs négatifs sur l'axe réel.

    **Zéros non triviaux** : conjecturés sur la droite critique Re(s) = 1/2 (hypothèse de Riemann).
    """)

    col1, col2 = st.columns(2)
    with col1:
        im_min = st.number_input(
            "Im(s) min",
            value=float(min(riemann_viz.ZEROS_NON_TRIVIAUX_IM)),
            step=1.0,
            key="im_min",
        )
    with col2:
        im_max = st.number_input(
            "Im(s) max",
            value=float(max(riemann_viz.ZEROS_NON_TRIVIAUX_IM)),
            step=1.0,
            key="im_max",
        )

    nb_triviaux = st.slider("Nombre de zéros triviaux affichés", 5, 30, 20, key="nb_triviaux")

    if im_min > im_max:
        st.warning("Im(s) min doit être ≤ Im(s) max.")
    else:
        non_triviaux = riemann_viz.zeros_non_triviaux(im_min, im_max)
        st.write(
            f"Zéros non triviaux dans l'intervalle : **{len(non_triviaux)}** "
            f"/ {len(riemann_viz.ZEROS_NON_TRIVIAUX_IM)}"
        )

        with st.expander("Voir les coordonnées des zéros"):
            triviaux = riemann_viz.zeros_triviaux(nb_triviaux)
            st.markdown("**Zéros triviaux** (Re(s), Im(s)) :")
            st.write([(float(x), 0.0) for x in triviaux])
            st.markdown("**Zéros non triviaux** (Re(s), Im(s), rang) :")
            st.write([(z[0], z[1], z[2]) for z in non_triviaux])

        tab_2d, tab_3d = st.tabs(["Visualisation 2D", "Visualisation 3D"])

        with tab_2d:
            animer = st.checkbox("Animation progressive des zéros non triviaux", value=False, key="anim_2d")
            fig_2d = riemann_viz.figure_2d(im_min, im_max, nb_triviaux, animer=animer)
            st.plotly_chart(fig_2d, use_container_width=True)
            st.caption(
                "Plan complexe : carrés bleus = zéros triviaux, points colorés = zéros non triviaux "
                "sur la droite critique. La couleur indique le rang du zéro."
            )

        with tab_3d:
            fig_3d = riemann_viz.figure_3d(im_min, im_max, nb_triviaux)
            st.plotly_chart(fig_3d, use_container_width=True)
            st.caption(
                "Vue 3D interactive : faites pivoter avec la souris. "
                "L'axe vertical (rang n) montre l'ordre des zéros non triviaux."
            )

        st.markdown("""
        ---
        ### À propos de la fonction zêta de Riemann
        La fonction zêta de Riemann \(\zeta(s)\) est définie pour \(\mathrm{Re}(s) > 1\) par :
        \[ \zeta(s) = \sum_{n=1}^{\infty} \frac{1}{n^s} \]

        - **Zéros triviaux** : \(-2, -4, -6, \ldots\)
        - **Zéros non triviaux** : conjecturés sur \(\mathrm{Re}(s) = \frac{1}{2}\)
        - **Hypothèse de Riemann** : tous les zéros non triviaux ont \(\mathrm{Re}(s) = \frac{1}{2}\)

        Ces zéros sont liés à la répartition des nombres premiers.
        """)
elif graphe == "Affichage des entiers et des nombres premiers":
    st.markdown("""
    Affiche tous les entiers dans l'intervalle choisi, les nombres premiers sont en <span style='color:red'>rouge</span>.
    """, unsafe_allow_html=True)
    col1, col2 = st.columns(2)
    with col1:
        min_aff = st.number_input("Borne minimale", value=int(nombres_premiers[0]), step=1, key="min_aff")
    with col2:
        max_aff = st.number_input("Borne maximale", value=int(nombres_premiers[0])+100, step=1, key="max_aff")
    intervalle_aff = st.slider(
        "(Optionnel) Ajustez l'intervalle avec le curseur",
        min_value=int(nombres_premiers[0]),
        max_value=int(nombres_premiers[-1]),
        value=(min_aff, max_aff),
        step=1,
        key="slider_aff"
    )
    if intervalle_aff != (min_aff, max_aff):
        min_aff, max_aff = intervalle_aff
    if min_aff > max_aff:
        st.warning("La borne minimale doit être inférieure ou égale à la borne maximale.")
    else:
        entiers = np.arange(min_aff, max_aff+1)
        premiers_set = set(nombres_premiers[(nombres_premiers >= min_aff) & (nombres_premiers <= max_aff)])
        # Affichage dans une div scrollable
        html = """
        <div style='font-family:monospace;font-size:18px;line-height:1.6;max-height:400px;overflow-y:auto;border:1px solid #ddd;padding:8px;background:#fafafa;'>
        """
        for i, n in enumerate(entiers):
            color = 'red' if n in premiers_set else 'black'
            html += f"<span style='color:{color};'>{n}</span> "
            if (i+1) % 10 == 0:
                html += "<br>"
        html += "</div>"
        st.markdown(html, unsafe_allow_html=True)

st.info("Lancez ce dashboard avec :\n\n    python -m streamlit run analyse.py\n\nDépendances : streamlit, matplotlib, numpy, plotly") 