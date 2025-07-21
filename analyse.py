import struct
import os
import numpy as np
import streamlit as st
import matplotlib.pyplot as plt

# Fonction pour lire les nombres premiers depuis le fichier binaire
def lire_nombres_premiers(fichier):
    if not os.path.exists(fichier):
        st.error(f"Fichier '{fichier}' introuvable.")
        return np.array([])
    with open(fichier, "rb") as f:
        data = f.read()
        count = len(data) // 4  # 4 octets par entier
        return np.array(struct.unpack(f'<{count}I', data), dtype=np.uint64)

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
    **Zéros triviaux** : -2, -4, -6, ... (nombres pairs négatifs)
    
    **Zéros non triviaux** : zéros sur la droite critique Re(s) = 1/2 (conjecture de Riemann)
    """)
    # Zéros triviaux
    zeros_triviaux = np.arange(-2, -40, -2)
    # Zéros non triviaux connus (20 premiers, partie imaginaire)
    zeros_non_triviaux_im = [14.134725, 21.022040, 25.010858, 30.424876, 32.935062,
        37.586178, 40.918719, 43.327073, 48.005150, 49.773832,
        52.970321, 56.446247, 59.347044, 60.831780, 65.112544,
        67.079811, 69.546402, 72.067158, 75.704690, 77.144840]
    zeros_non_triviaux = [0.5 + 1j*y for y in zeros_non_triviaux_im]

    # Sélection de l'intervalle d'ordonnée (Im(s))
    st.write("Filtrer les zéros non triviaux selon l'ordonnée Im(s) : (seuls les 20 premiers zéros non triviaux sont disponibles)")
    col1, col2 = st.columns(2)
    with col1:
        im_min = st.number_input("Im(s) min", value=float(min(zeros_non_triviaux_im)), step=1.0, key="im_min")
    with col2:
        im_max = st.number_input("Im(s) max", value=float(max(zeros_non_triviaux_im)), step=1.0, key="im_max")
    if im_min > im_max:
        st.warning("Im(s) min doit être ≤ Im(s) max.")
    # Filtrage des zéros non triviaux
    zeros_non_triviaux_filtrés = [z for z in zeros_non_triviaux if im_min <= z.imag <= im_max]

    # Affichage des coordonnées
    with st.expander("Voir les coordonnées des zéros"):
        st.markdown("**Zéros triviaux** (Re(s), Im(s)) :")
        st.write([ (float(x), 0.0) for x in zeros_triviaux ])
        st.markdown("**Zéros non triviaux** (Re(s), Im(s)) :")
        st.write([ (z.real, z.imag) for z in zeros_non_triviaux_filtrés ])

    # Préparation du graphique amélioré
    fig, ax = plt.subplots(figsize=(8,6))
    # Triviaux : points sur l'axe réel (toujours affichés)
    ax.scatter(zeros_triviaux, [0]*len(zeros_triviaux), color='royalblue', label='Triviaux', s=120, marker='s', edgecolor='black', zorder=10)
    for x in zeros_triviaux:
        ax.annotate(f"({x}, 0)", (x, 0), textcoords="offset points", xytext=(0,10), ha='center', fontsize=10, color='royalblue', bbox=dict(boxstyle='round,pad=0.2', fc='white', ec='none', alpha=0.8))
    # Non triviaux filtrés : points sur la droite critique
    ax.scatter([z.real for z in zeros_non_triviaux_filtrés], [z.imag for z in zeros_non_triviaux_filtrés], color='crimson', label='Non triviaux', s=80, marker='o', edgecolor='black', zorder=4)
    for z in zeros_non_triviaux_filtrés:
        ax.annotate(f"({z.real:.1f}, {z.imag:.2f})", (z.real, z.imag), textcoords="offset points", xytext=(10,0), ha='left', fontsize=9, color='crimson')
    ax.axvline(0.5, color='gray', linestyle='--', alpha=0.5, label='Droite critique Re(s)=1/2', zorder=1)
    ax.set_xlabel('Re(s)', fontsize=13)
    ax.set_ylabel('Im(s)', fontsize=13)
    ax.set_title('Zéros triviaux et non triviaux de la fonction zêta de Riemann', fontsize=15, pad=15)
    ax.legend(fontsize=12)
    ax.grid(True, alpha=0.3, linestyle=':')
    ax.set_facecolor('#f7f7fa')
    # Limites du graphique
    ax.set_xlim(-40, 2)
    # Forcer l'axe Y à inclure 0 pour voir les triviaux
    y_min = min(im_min, min(zeros_non_triviaux_im)-5, -5)
    y_max = max(im_max, max(zeros_non_triviaux_im)+5, 5)
    ax.set_ylim(y_min, y_max)
    st.pyplot(fig)

    # Explications mathématiques
    st.markdown("""
    ---
    ### À propos de la fonction zêta de Riemann
    La fonction zêta de Riemann \(\zeta(s)\) est définie pour \(\mathrm{Re}(s) > 1\) par la série :
    \[ \zeta(s) = \sum_{n=1}^{\infty} \frac{1}{n^s} \]
    Elle admet un prolongement analytique sur \(\mathbb{C} \setminus \{1\}\).
    
    - **Zéros triviaux** : ce sont les entiers pairs négatifs \(-2, -4, -6, \ldots\)
    - **Zéros non triviaux** : conjecturés tous sur la droite \(\mathrm{Re}(s) = 1/2\) (conjecture de Riemann)
    - **Conjecture de Riemann** : tous les zéros non triviaux de \(\zeta(s)\) ont une partie réelle égale à 1/2.
    
    Les zéros non triviaux sont fondamentaux en théorie des nombres, notamment pour la répartition des nombres premiers.
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

st.info("Lancez ce dashboard avec :\n\n    streamlit run analyse.py\n\nDépendances : streamlit, matplotlib, numpy") 